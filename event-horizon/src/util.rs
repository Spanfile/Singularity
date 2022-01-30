pub mod request_callback_error;
pub mod round_duration;

use crate::error::{EvhError, EvhResult};
use chrono::{DateTime, Local};
use cron_clock::Schedule;

// so today I learned the library I use to parse cron expressions also expects specifiers for seconds and years. are
// they even a thing in any cron implementation? anyways, cursed solution is to just slap the match-all specifiers
// for them around the given expression
pub fn expand_cron_expression(expression: &str) -> String {
    format!("* {} *", expression)
}

pub fn next_cron_run(expression: &str) -> EvhResult<DateTime<Local>> {
    let schedule: Schedule = expand_cron_expression(expression)
        .parse()
        .map_err(EvhError::InvalidCronSchedule)?;
    let next = schedule
        .upcoming_owned(Local)
        .next()
        .expect("no upcoming datetimes in schedule");

    Ok(next)
}