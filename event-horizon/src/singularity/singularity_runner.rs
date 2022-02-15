use crate::error::{EvhError, EvhResult};
use nanoid::nanoid;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct SingularityRunner(Mutex<RunnerState>);

#[derive(Debug)]
struct RunnerState {
    currently_running: Option<SingularityRunningState>,
}

#[derive(Debug)]
enum SingularityRunningState {
    Running(String),
    Finished(String),
}

#[derive(Debug)]
pub enum CurrentlyRunningSingularity {
    Running,
    Finished,
}

impl SingularityRunner {
    pub fn new() -> Self {
        Self(Mutex::new(RunnerState {
            currently_running: None,
        }))
    }

    pub async fn get_currently_running(&self) -> Option<CurrentlyRunningSingularity> {
        let state = self.0.lock().await;
        state.currently_running.as_ref().map(|s| match s {
            SingularityRunningState::Running(_) => CurrentlyRunningSingularity::Running,
            SingularityRunningState::Finished(_) => CurrentlyRunningSingularity::Finished,
        })
    }

    pub async fn run(&self) -> EvhResult<()> {
        let mut state = self.0.lock().await;

        if state.currently_running.is_some() {
            return Err(EvhError::SingularityAlreadyRunning);
        }

        let id = nanoid!();
        state.currently_running = Some(SingularityRunningState::Running(id));

        Ok(())
    }
}
