mod config;
mod logging;

use config::Config;
use crossbeam_utils::atomic::AtomicCell;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::*;
use num_format::{SystemLocale, ToFormattedString};
use singularity::{Progress, Singularity, HTTP_CONNECT_TIMEOUT};
use std::{fmt::Display, path::PathBuf, str::FromStr, time::Instant};
use structopt::StructOpt;

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Copy, Clone)]
struct ConnectTimeout(u64);

impl Default for ConnectTimeout {
    fn default() -> Self {
        Self(HTTP_CONNECT_TIMEOUT)
    }
}

impl FromStr for ConnectTimeout {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Display for ConnectTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = APP_NAME, author, about
)]
struct Opt {
    /// Enable verbose logging
    #[structopt(short, long)]
    verbose: bool,
    /// Custom path to the app's configuration file. By default the app will use the system-specific user configuration
    /// directory.
    #[structopt(short, long)]
    config: Option<PathBuf>,
    /// The timeout to wait for HTTP requests to succeed in milliseconds.
    #[structopt(default_value, short, long)]
    timeout: ConnectTimeout,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    setup_logging(&opt)?;
    let cfg = load_config(&opt)?;

    debug!("{:?}", opt);
    debug!("{:?}", cfg);

    if cfg.adlist.is_empty() {
        warn!("No adlists configured. Please edit the configuration file and add one or more adlists.");
        return Ok(());
    }

    if cfg.output.is_empty() {
        warn!("No outputs configured. Please edit the configuration file and add one or more outputs.");
        return Ok(());
    }

    let adlists = cfg.adlist.len();

    let singularity = Singularity::builder()
        .add_many_adlists(cfg.adlist)
        .add_outputs_from_configs(cfg.output)
        .whitelist_many_domains(cfg.whitelist)
        .http_timeout(opt.timeout.0)
        .build();

    let count = AtomicCell::<usize>::new(0);
    let start = Instant::now();

    singularity
        .progress_callback(|progress| match progress {
            Progress::BeginAdlistRead { source, length } => {
                if let Some(len) = length {
                    info!("Reading {} with length {}", source, len)
                } else {
                    info!("Reading {} with indeterminate length", source)
                }
            }
            Progress::DomainWritten(_) => {
                count.fetch_add(1);
            }

            Progress::WhitelistedDomainIgnored { source, domain } => {
                info!("Ignoring whitelisted domain {} from {}", domain, source)
            }
            Progress::AllMatchingLineIgnored {
                source,
                line_number,
                line,
            } => warn!(
                "Line {} in {} parsed to an all-matching entry ({}), so it was ignored",
                line_number, source, line
            ),

            _ => (),
        })
        .run()?;

    let locale = SystemLocale::default().expect("failed to get system locale");
    info!(
        "Read {} domains from {} sources in {}s",
        count.into_inner().to_formatted_string(&locale),
        adlists,
        start.elapsed().as_secs_f32()
    );

    Ok(())
}

fn setup_logging(opt: &Opt) -> anyhow::Result<()> {
    logging::setup_logging(if opt.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    })?;
    Ok(())
}

fn load_config(opt: &Opt) -> anyhow::Result<Config> {
    Ok(match opt.config.as_deref() {
        Some(path) => confy::load_path(path)?,
        None => confy::load(APP_NAME)?,
    })
}
