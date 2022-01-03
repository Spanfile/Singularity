use fern::Dispatch;
use log::LevelFilter;

pub fn setup_logging(log_level: LevelFilter) -> anyhow::Result<()> {
    Dispatch::new()
        .format(move |out, msg, record| out.finish(format_args!("{} {}", record.level(), msg)))
        .level(log_level)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
