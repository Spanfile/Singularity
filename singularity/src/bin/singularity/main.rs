mod config;
mod logging;

use config::Config;
use crossbeam_utils::{atomic::AtomicCell, thread};
use dashmap::DashMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::*;
use num_format::{SystemLocale, ToFormattedString};
use singularity::{Progress, Singularity, SingularityError, HTTP_CONNECT_TIMEOUT};
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

    let adlists_len = cfg.adlist.len();

    // Singularity needs at least one adlist and one output to work. the builder will return an error if either are
    // missing. multiple adlists and outputs can be added at once, or the methods `.add_adlist()` and `.add_output()`
    // may be used to add single ones
    let builder = Singularity::builder()
        .add_many_adlists(cfg.adlist)
        .add_many_outputs(cfg.output)
        .whitelist_many_domains(cfg.whitelist)
        .http_timeout(opt.timeout.0);

    // gracefully handle the two possible error cases from the builder
    let singularity = match builder.build() {
        Ok(singularity) => singularity,
        Err(SingularityError::NoAdlists) => {
            warn!("No adlists configured. Please edit the configuration file and add one or more adlists.");
            return Ok(());
        }
        Err(SingularityError::NoOutputs) => {
            warn!("No outputs configured. Please edit the configuration file and add one or more outputs.");
            return Ok(());
        }
        Err(e) => panic!("unexpected error while building Singularity: {}", e),
    };

    let mp = MultiProgress::new();
    let domain_spinner = mp.add(ProgressBar::new_spinner());
    domain_spinner.set_style(
        ProgressStyle::default_spinner().template("{spinner} [{elapsed_precise}] {pos} domains read so far..."),
    );
    domain_spinner.enable_steady_tick(100);
    domain_spinner.set_draw_delta(500);

    // use a thread scope to spawn two threads: one to handle running Singularity, and one to join and wait the
    // MultiProgress. the scope is used to ensure the Singularity thread doesn't outlive the Singularity object
    thread::scope(|s| -> anyhow::Result<()> {
        s.spawn(|_| -> anyhow::Result<()> {
            let count = AtomicCell::<usize>::new(0);
            let start = Instant::now();
            let pbs = DashMap::new();

            // while running, Singularity will report progress back using a callback. this callback may borrow objects
            // from the outer scope so long its lifetime doesn't exceed that of Singularity's. this is achieved by using
            // the thread scope
            singularity
                .progress_callback(|progress| match progress {
                    // this status is returned once for each adlist when they are began reading. in some cases the
                    // adlist's total length cannot be determined ahead of time; display a progress bar if it is known,
                    // otherwise display a spinner
                    Progress::BeginAdlistRead { source, length } => {
                        let pb = mp.add(ProgressBar::new(0));

                        if let Some(len) = length {
                            domain_spinner.println(format!("INFO Reading {} with length {}", source, len));

                            pb.set_style(
                                ProgressStyle::default_bar()
                                    .template("[{elapsed_precise}] [{bar:40}] {bytes}/{total_bytes} ({bytes_per_sec})")
                                    .progress_chars("=> "),
                            );
                            pb.set_length(len);
                        } else {
                            domain_spinner.println(format!("INFO Reading {} with indeterminate length", source));

                            pb.set_style(
                                ProgressStyle::default_spinner()
                                    .template("{spinner} [{elapsed_precise}] {bytes} ({bytes_per_sec})"),
                            );
                        }

                        pbs.insert(source.to_string(), pb);
                    }

                    // this status is returned periodically when an adlist is being read. contains the amount of bytes
                    // read so far, and how many more bytes have been read since the previous read progress update
                    Progress::ReadProgress {
                        source,
                        bytes: _,
                        delta,
                    } => pbs.get(source).expect("progress bar missing from pbs").inc(delta),

                    // an adlist has been finished reading. finish its corresponding progress bar
                    Progress::FinishAdlistRead { source } => pbs
                        .get(source)
                        .expect("progress bar missing from pbs")
                        .finish_and_clear(),

                    // a domain was succesfully read, parsed and not ignored from an adlist, and it was written to each
                    // output. individual output writes may fail, in which case an OutputWriteFailed progress update is
                    // raised for each failed one
                    Progress::DomainWritten(_) => {
                        count.fetch_add(1);
                        domain_spinner.inc(1);
                    }

                    // a domain from some source was in the whitelist and was ignored
                    Progress::WhitelistedDomainIgnored { source, domain } => pbs
                        .get(source)
                        .expect("progress bar missing from pbs")
                        .println(format!("INFO Ignoring whitelisted domain {} from {}", domain, source)),

                    // a domain was parsed into an all-matching entry, such as '.', so it was ignored
                    Progress::AllMatchingLineIgnored {
                        source,
                        line_number,
                        line,
                    } => pbs.get(source).expect("progress bar missing from pbs").println(format!(
                        "WARN Line {} in {} parsed to an all-matching entry ({}), so it was ignored",
                        line_number, source, line
                    )),

                    // a line in an adlist was invalid, likely because it was not valid UTF-8. that line is ignored and
                    // the adlist is continued to be read
                    Progress::InvalidLine {
                        source,
                        line_number,
                        reason,
                    } => pbs.get(source).expect("progress bar missing from pbs").println(format!(
                        "WARN Line {} in {} is invalid: {}",
                        line_number, source, reason
                    )),

                    // reading an adlist failed. the adlist's reading is aborted
                    Progress::ReadingAdlistFailed { source, reason } => pbs
                        .get(source)
                        .expect("progress bar missing from pbs")
                        .finish_with_message(format!("ERROR Reading adlist '{}' failed: {}", source, reason)),

                    // writing a domain or finalising an output failed
                    Progress::OutputWriteFailed { output_dest, reason } => domain_spinner.println(format!(
                        "ERROR Writing to output '{}' failed: {}",
                        output_dest.display(),
                        reason
                    )),
                })
                .run()?;

            let locale = SystemLocale::default().expect("failed to get system locale");
            domain_spinner.println(format!(
                "INFO Read {} domains from {} sources in {}s",
                count.into_inner().to_formatted_string(&locale),
                adlists_len,
                start.elapsed().as_secs_f32()
            ));
            domain_spinner.finish_and_clear();

            Ok(())
        });

        s.spawn(|_| -> anyhow::Result<()> {
            mp.join_and_clear()?;
            Ok(())
        });

        Ok(())
    })
    .unwrap()?;

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
