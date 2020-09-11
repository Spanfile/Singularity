use fern::Dispatch;
use log::LevelFilter;
use std::time::Instant;

pub fn setup_logging(log_level: LevelFilter) -> anyhow::Result<()> {
    let start = Instant::now();

    Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "{: >11.3} {: >5} {}",
                // "[{} UTC] [{}] {}",
                // chrono::Utc::now().format(time_format),
                start.elapsed().as_secs_f32(),
                record.level(),
                msg
            ))
        })
        .level(log_level)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
