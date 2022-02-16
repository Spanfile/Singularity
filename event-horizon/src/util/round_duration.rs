use std::time::Duration;

pub trait RoundDuration {
    fn round_to_minutes(self) -> Self;
    fn round_to_seconds(self) -> Self;
}

impl RoundDuration for Duration {
    // these are an intentional loss of precision

    fn round_to_minutes(self) -> Self {
        Self::from_secs((self.as_secs() / 60) * 60)
    }

    fn round_to_seconds(self) -> Self {
        Self::from_secs(self.as_secs())
    }
}
