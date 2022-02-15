#[derive(Debug)]
pub struct SingularityRunner {}

pub enum CurrentlyRunningSingularity {
    Running,
    Finished,
}

impl SingularityRunner {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_currently_running(&self) -> Option<CurrentlyRunningSingularity> {
        Some(CurrentlyRunningSingularity::Finished)
    }
}
