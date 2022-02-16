pub mod history;

use super::singularity_config::SingularityConfig;
use crate::{
    database::DbPool,
    error::{EvhError, EvhResult},
};
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
        let state = self.0.lock().expect("runner state mutex poisoned");
        state.currently_running.as_ref().map(|s| match s {
            SingularityRunningState::Running(..) => CurrentlyRunningSingularity::Running,
            SingularityRunningState::Finished(_) => CurrentlyRunningSingularity::Finished,
        })
    }

    pub fn run(&self, cfg: SingularityConfig, pool: Arc<DbPool>) -> EvhResult<()> {
        {
            // scope to drop the guard and unlock the mutex when this is done
            let state = self.0.lock().expect("runner state mutex poisoned");
            if let Some(SingularityRunningState::Running(..)) = state.currently_running {
                return Err(EvhError::SingularityAlreadyRunning);
            }
        }

        self.spawn_runner_thread(cfg, pool)?;
        Ok(())
    }

    pub fn terminate(&self) -> EvhResult<()> {
        todo!()
    }

    fn spawn_runner_thread(&self, cfg: SingularityConfig, pool: Arc<DbPool>) -> EvhResult<()> {
        let id = nanoid!();

        // move clones of the id and the state arc to the runner thread
        let _id = id.clone();
        let _state = Arc::clone(&self.0);
        let runner_handle = std::thread::spawn(move || {
            let res = runner_thread(&_id, cfg, pool);
            debug!("Singularity {}: runner thread finished with result: {:?}", _id, res);

            match res {
                Ok(_) => {
                    info!("Singularity run ID {} finished running succesfully", _id);
                }
                Err(e) => {
                    error!("Singularity run ID {} returned error: {}", _id, e);
                }
            }

            let mut state = _state.lock().expect("runner state mutex poisoned");
            state.currently_running = Some(SingularityRunningState::Finished(_id));
        });

        info!("Running Singularity. Run ID: {}", id);
        let mut state = self.0.lock().expect("runner state mutex poisoned");
        state.currently_running = Some(SingularityRunningState::Running(id, runner_handle));

        Ok(())
    }
}

fn runner_thread(id: &str, cfg: SingularityConfig, pool: Arc<DbPool>) -> EvhResult<()> {
    debug!("Singularity {}: runner thread starting", id);

    let mut conn = pool.get()?;
    cfg.set_last_run_and_clear_dirty(&mut conn)?;

    let (adlists, outputs, whitelist, http_timeout) = cfg.get_singularity_builder_config(&mut conn)?;
    debug!(
        "Singularity {}: {} adlists, {} outputs, {} whitelisted domains",
        id,
        adlists.len(),
        outputs.len(),
        whitelist.len()
    );

    let singularity = Singularity::builder()
        .add_many_adlists(adlists)
        .add_many_outputs(outputs)
        .whitelist_many_domains(whitelist)
        .http_timeout(http_timeout as u64)
        .build()?;

    let domain_count = AtomicCell::<usize>::new(0);
    let start = Instant::now();

    singularity
        .progress_callback(|prog| match prog {
            Progress::BeginAdlistRead { source, length } => debug!(
                "Singularity {}: beginning adlist read from {} with length {:?}",
                id, source, length
            ),

            Progress::ReadProgress { source, bytes, delta } => {
                // TODO: keep track of individual read source read speeds
            }

            Progress::FinishAdlistRead { source } => debug!("Singularity {}: finished reading {}", id, source),

            Progress::DomainWritten(_) => {
                domain_count.fetch_add(1);
            }

            Progress::WhitelistedDomainIgnored { source, domain } => {
                debug!("Singularity {}: ignored domain {} from {}", id, domain, source)
            }

            Progress::AllMatchingLineIgnored {
                source,
                line_number,
                line,
            } => warn!(
                "Singularity {}: line {} in {} is all-matching: '{}'",
                id, line_number, source, line
            ),

            Progress::InvalidLine {
                source,
                line_number,
                reason,
            } => warn!(
                "Singularity {}: line {} in {} is invalid: '{}'",
                id, line_number, source, reason
            ),

            Progress::ReadingAdlistFailed { source, reason } => {
                warn!("Singularity {}: failed to read adlist {}: {}", id, source, reason)
            }

            Progress::OutputWriteFailed { output_dest, reason } => {
                warn!(
                    "Singularity {}: failed to write to output {}: {}",
                    id,
                    output_dest.display(),
                    reason
                )
            }
        })
        .run()?;

    debug!(
        "Singularity {}: {} domains read, elapsed {}s",
        id,
        domain_count.load(),
        start.elapsed().as_secs_f32()
    );
    Ok(())
}
