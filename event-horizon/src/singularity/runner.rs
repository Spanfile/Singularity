pub mod history;

use self::history::RunnerHistory;
use super::singularity_config::SingularityConfig;
use crate::{
    config::EvhConfig,
    database::DbPool,
    error::{EvhError, EvhResult},
};
use chrono::Local;
use crossbeam_utils::atomic::AtomicCell;
use log::*;
use nanoid::nanoid;
use singularity::{Progress, Singularity};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Instant,
};

// TODO: there's two levels of indirection because of this Arc, because actix sticks this entire thing in one Arc and we
// stick the mutex in another. really it'd be better if we had access to actix's Arc here but how exactly would we do
// that?
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
    Finished,
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
            SingularityRunningState::Finished(_) => CurrentlyRunningSingularity::Finished,
        })
    }

    pub fn run(&self, cfg: SingularityConfig, pool: Arc<DbPool>, evh_cfg: Arc<EvhConfig>) -> EvhResult<()> {
        let state = self.0.lock().expect("runner state mutex is poisoned");
        if let Some(SingularityRunningState::Running(..)) = state.currently_running {
            return Err(EvhError::SingularityAlreadyRunning);
        }

        // drop the mutex guard to unlock the mutex and prevent deadlocks in the runner thread
        drop(state);

        self.spawn_runner_thread(cfg, evh_cfg, pool)?;
        Ok(())
    }

    pub fn terminate(&self) -> EvhResult<()> {
        todo!()
    }

    fn spawn_runner_thread(&self, cfg: SingularityConfig, evh_cfg: Arc<EvhConfig>, pool: Arc<DbPool>) -> EvhResult<()> {
        let id = nanoid!();

        // move clones of the id and the state arc to the runner thread
        let _id = id.clone();
        let _state = Arc::clone(&self.0);
        let runner_handle = std::thread::spawn(move || {
            let res = runner_thread(&_id, cfg, evh_cfg, pool);
            debug!("Singularity {}: runner thread finished with result: {:?}", _id, res);

            match res {
                Ok(_) => {
                    info!("Singularity run ID {} finished succesfully", _id);
                }
                Err(e) => {
                    error!("Singularity run ID {} returned error: {}", _id, e);
                }
            }

            let mut state = _state.lock().expect("runner state mutex is poisoned");
            state.currently_running = Some(SingularityRunningState::Finished(_id));
        });

        info!("Running Singularity. Run ID: {}", id);
        let mut state = self.0.lock().expect("runner state mutex is poisoned");
        state.currently_running = Some(SingularityRunningState::Running(id, runner_handle));

        Ok(())
    }
}

fn runner_thread(id: &str, cfg: SingularityConfig, evh_cfg: Arc<EvhConfig>, pool: Arc<DbPool>) -> EvhResult<()> {
    let start = Instant::now();
    let now = Local::now();
    let mut history = RunnerHistory::new(id, now);

    history.debug(start.elapsed().as_secs_f32(), "runner thread starting".to_string());

    let mut conn = pool.get()?;
    cfg.set_last_run(&mut conn, now)?;

    let (adlists, outputs, whitelist, http_timeout) = cfg.get_singularity_builder_config(&mut conn)?;
    history.debug(
        start.elapsed().as_secs_f32(),
        format!(
            "{} adlists, {} outputs, {} whitelisted domains",
            adlists.len(),
            outputs.len(),
            whitelist.len()
        ),
    );

    // get rid of the database connection, it's not needed during the run
    drop(conn);

    let singularity = Singularity::builder()
        .add_many_adlists(adlists)
        .add_many_outputs(outputs)
        .whitelist_many_domains(whitelist)
        .http_timeout(http_timeout as u64)
        .build()?;

    // move the history into a mutex in an arc so the callback can access it
    let history = Arc::new(Mutex::new(RunnerHistory::new(id, now)));
    let domain_count = AtomicCell::<usize>::new(0);

    singularity
        .progress_callback(|prog| match prog {
            Progress::BeginAdlistRead { source, length } => history.lock().expect("history mutex is poisoned").info(
                start.elapsed().as_secs_f32(),
                format!("beginning adlist read from {} with length {:?}", source, length),
            ),

            Progress::ReadProgress { source, bytes, delta } => {
                // TODO: keep track of individual read source read speeds
            }

            Progress::FinishAdlistRead { source } => history
                .lock()
                .expect("history mutex is poisoned")
                .info(start.elapsed().as_secs_f32(), format!("finished reading {}", source)),

            Progress::DomainWritten(_) => {
                domain_count.fetch_add(1);
            }

            Progress::WhitelistedDomainIgnored { source, domain } => {
                history.lock().expect("history mutex is poisoned").debug(
                    start.elapsed().as_secs_f32(),
                    format!("ignored domain {} from {}", domain, source),
                )
            }

            Progress::AllMatchingLineIgnored {
                source,
                line_number,
                line,
            } => history.lock().expect("history mutex is poisoned").warn(
                start.elapsed().as_secs_f32(),
                format!("line {} in {} is all-matching: '{}'", line_number, source, line),
            ),

            Progress::InvalidLine {
                source,
                line_number,
                reason,
            } => history.lock().expect("history mutex is poisoned").warn(
                start.elapsed().as_secs_f32(),
                format!("line {} in {} is invalid: '{}'", line_number, source, reason),
            ),

            Progress::ReadingAdlistFailed { source, reason } => {
                history.lock().expect("history mutex is poisoned").error(
                    start.elapsed().as_secs_f32(),
                    format!("failed to read adlist {}: {}", source, reason),
                )
            }

            Progress::OutputWriteFailed { output_dest, reason } => {
                history.lock().expect("history mutex is poisoned").error(
                    start.elapsed().as_secs_f32(),
                    format!("failed to write to output {}: {}", output_dest.display(), reason),
                )
            }
        })
        .run()?;

    let mut history = history.lock().expect("history mutex is poisoned");
    history.info(
        start.elapsed().as_secs_f32(),
        format!(
            "{} domains read, elapsed {}s",
            domain_count.load(),
            start.elapsed().as_secs_f32()
        ),
    );

    let mut conn = pool.get()?;
    history.save(&mut conn, &evh_cfg)?;
    cfg.set_dirty(&mut conn, false)?;

    Ok(())
}
