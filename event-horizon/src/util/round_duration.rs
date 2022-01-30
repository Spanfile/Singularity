use std::time::Duration;

pub trait RoundDuration {
    fn round_to_minutes(self) -> Self;
}

impl RoundDuration for Duration {
    fn round_to_minutes(self) -> Self {
        // this is an intentional loss of precision
        Self::from_secs((self.as_secs() / 60) * 60)
    }
}
