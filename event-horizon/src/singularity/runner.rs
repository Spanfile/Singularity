pub mod history;

use self::history::RunnerHistory;
use super::singularity_config::SingularityConfig;
use crate::{
    config::EvhConfig,
    database::{models::SingularityRunHistoryResult, DbPool},
    error::{EvhError, EvhResult},
    logging::LogLevel,
    util::estimate::Estimate,
};
use chrono::Local;
use crossbeam_utils::atomic::AtomicCell;
use dashmap::DashMap;
use human_bytes::human_bytes;
use log::*;
use nanoid::nanoid;
use singularity::{Progress, Singularity};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Instant,
};

// TODO: there's two levels of indirection because of this Arc, because actix sticks this entire thing in one Arc and we
// stick the mutex in another. the Arc is required so the state can be shared with the runner thread later. really it'd
// be better if we had access to actix's Arc here but how exactly would we do that?
#[derive(Debug)]
pub struct SingularityRunner(Arc<Mutex<RunnerState>>);

#[derive(Debug)]
struct RunnerState {
    currently_running: Option<SingularityRunningState>,
}

#[derive(Debug)]
enum SingularityRunningState {
    Running(String, JoinHandle<()>),
    Finished(String),
}

#[derive(Debug)]
pub enum CurrentlyRunningSingularity {
    Running,
    Finished(String),
}

#[derive(Debug, Default)]
struct AdlistTracker {
    length: Option<u64>,
    bytes_read: u64,
    estimate: Estimate<16>,
}

impl SingularityRunner {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(RunnerState {
            currently_running: None,
        })))
    }

    pub async fn get_currently_running(&self) -> Option<CurrentlyRunningSingularity> {
        let state = self.0.lock().expect("runner state mutex is poisoned");
        state.currently_running.as_ref().map(|s| match s {
            SingularityRunningState::Running(..) => CurrentlyRunningSingularity::Running,
            SingularityRunningState::Finished(id) => CurrentlyRunningSingularity::Finished(id.to_owned()),
        })
    }

    pub fn run(&self, cfg: SingularityConfig, evh_cfg: Arc<EvhConfig>, db_pool: Arc<DbPool>) -> EvhResult<()> {
        let state = self.0.lock().expect("runner state mutex is poisoned");
        if let Some(SingularityRunningState::Running(..)) = state.currently_running {
            return Err(EvhError::SingularityRunning);
        }

        // drop the mutex guard to unlock the mutex and prevent deadlocks in the runner thread
        drop(state);

        self.spawn_runner_thread(cfg, evh_cfg, db_pool)?;
        Ok(())
    }

    pub fn terminate(&self) -> EvhResult<()> {
        todo!()
    }

    fn spawn_runner_thread(
        &self,
        cfg: SingularityConfig,
        evh_cfg: Arc<EvhConfig>,
        db_pool: Arc<DbPool>,
    ) -> EvhResult<()> {
        let id = nanoid!();

        // move clones of the id and the state arc to the runner thread
        let _id = id.clone();
        let _state = Arc::clone(&self.0);
        let runner_handle = std::thread::spawn(move || {
            let now = Local::now();
            let history = Arc::new(Mutex::new(RunnerHistory::new(&_id, now)));

            let res = runner_thread(cfg, Arc::clone(&history), Arc::clone(&db_pool));

            let mut _history = history.lock().expect("history mutex is poisoned");
            _history.debug(format!("Runner thread finished with result: {:?}", res));

            let run_result = match res {
                Ok(_) => {
                    let mut events = _history.events().iter();
                    loop {
                        match events.next().map(|e| e.severity()) {
                            Some(LogLevel::Error) => {
                                warn!("Singularity run ID {} finished succesfully with errors", _id);
                                break SingularityRunHistoryResult::SuccessWithErrors;
                            }
                            Some(LogLevel::Warn) => {
                                warn!("Singularity run ID {} finished succesfully with warnings", _id);
                                break SingularityRunHistoryResult::SuccessWithWarnings;
                            }
                            None => {
                                info!("Singularity run ID {} finished succesfully", _id);
                                break SingularityRunHistoryResult::Success;
                            }
                            _ => (),
                        }
                    }
                }
                Err(e) => {
                    error!("Singularity run ID {} returned error: {}", _id, e);
                    SingularityRunHistoryResult::Failed
                }
            };

            let mut state = _state.lock().expect("runner state mutex is poisoned");
            state.currently_running = Some(SingularityRunningState::Finished(_id));

            _history.set_result(run_result);

            if let Err(e) = db_pool.get().and_then(|mut conn| _history.save(&mut conn, &evh_cfg)) {
                error!("Failed to save run history: {}", e);
            }
        });

        info!("Running Singularity. Run ID: {}", id);
        let mut state = self.0.lock().expect("runner state mutex is poisoned");
        state.currently_running = Some(SingularityRunningState::Running(id, runner_handle));

        Ok(())
    }
}

fn runner_thread(cfg: SingularityConfig, history: Arc<Mutex<RunnerHistory>>, db_pool: Arc<DbPool>) -> EvhResult<()> {
    let start = Instant::now();

    let mut _history = history.lock().expect("history mutex is poisoned");
    _history.debug("Runner thread starting".to_string());

    let mut conn = db_pool.get()?;
    cfg.set_last_run(&mut conn, Local::now())?;

    let (adlists, outputs, whitelist, http_timeout) = cfg.get_singularity_builder_config(&mut conn)?;
    _history.debug(format!(
        "{} adlists, {} outputs, {} whitelisted domains",
        adlists.len(),
        outputs.len(),
        whitelist.len()
    ));

    // get rid of the database connection and unlock the history mutex, they're not needed during the run and would
    // cause deadlocks
    drop(_history);
    drop(conn);

    let singularity = Singularity::builder()
        .add_many_adlists(adlists)
        .add_many_outputs(outputs)
        .whitelist_many_domains(whitelist)
        .http_timeout(http_timeout as u64)
        .build()?;

    let domain_count = AtomicCell::<usize>::new(0);
    let adlist_trackers = DashMap::<String, AdlistTracker>::new();

    singularity
        .progress_callback(|prog| match prog {
            Progress::BeginAdlistRead { source, length } => {
                adlist_trackers.insert(
                    source.to_string(),
                    AdlistTracker {
                        length,
                        ..Default::default()
                    },
                );

                history.lock().expect("history mutex is poisoned").info(format!(
                    "Beginning adlist read from {} with length {:?}",
                    source, length
                ));
            }

            Progress::ReadProgress {
                source,
                bytes,
                delta: _,
            } => {
                let mut tracker = adlist_trackers.get_mut(source).expect("missing adlist tracker");
                tracker.bytes_read = bytes;
                tracker.estimate.step(bytes);
            }

            Progress::FinishAdlistRead { source } => history
                .lock()
                .expect("history mutex is poisoned")
                .info(format!("Finished reading {}", source)),

            Progress::DomainWritten(_) => {
                domain_count.fetch_add(1);
            }

            Progress::WhitelistedDomainIgnored { source, domain } => history
                .lock()
                .expect("history mutex is poisoned")
                .debug(format!("Ignored domain {} from {}", domain, source)),

            Progress::AllMatchingLineIgnored {
                source,
                line_number,
                line,
            } => history.lock().expect("history mutex is poisoned").warn(format!(
                "Line {} in {} is all-matching: '{}'",
                line_number, source, line
            )),

            Progress::InvalidLine {
                source,
                line_number,
                reason,
            } => history
                .lock()
                .expect("history mutex is poisoned")
                .warn(format!("Line {} in {} is invalid: '{}'", line_number, source, reason)),

            Progress::ReadingAdlistFailed { source, reason } => history
                .lock()
                .expect("history mutex is poisoned")
                .warn(format!("Failed to read adlist {}: {}", source, reason)),

            Progress::OutputWriteFailed { output_dest, reason } => {
                history.lock().expect("history mutex is poisoned").error(format!(
                    "Failed to write to output {}: {}",
                    output_dest.display(),
                    reason
                ))
            }
        })
        .run()?;

    let mut history = history.lock().expect("history mutex is poisoned");
    history.info(format!(
        "{} domains read, elapsed {}s",
        domain_count.load(),
        start.elapsed().as_secs_f32(),
    ));

    for (source, tracker) in adlist_trackers {
        history.info(format!(
            "{}: {} read at {}/s",
            source,
            human_bytes(tracker.bytes_read as f64),
            human_bytes(tracker.estimate.steps_per_second()),
        ));
    }

    Ok(())
}
