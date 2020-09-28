mod config;
mod error;
mod logging;
mod output;

use anyhow::Context;
use config::{AdlistFormat, Config};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use io::{BufRead, BufReader};
use log::*;
use num_format::{SystemLocale, ToFormattedString};
use output::Output;
use std::{
    fmt::Display,
    io,
    net::IpAddr,
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
};
use structopt::StructOpt;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const HTTP_CONNECT_TIMEOUT: u64 = 5_000;

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

    let mut outputs = Vec::new();
    for output_cfg in &cfg.output {
        let mut output = Output::from_config(output_cfg).with_context(|| "Failed to create output")?;
        output.write_primer()?;
        outputs.push(output);
    }

    let mb = MultiProgress::new();
    let download_style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40}] {bytes}/{total_bytes} ({bytes_per_sec})")
        .progress_chars("=> ");
    let spinner_style = ProgressStyle::default_spinner().template("{spinner} {pos} domains read so far...");

    let (tx, rx) = mpsc::sync_channel::<String>(1024);
    let count = Arc::new(AtomicUsize::new(0));
    let count_c = Arc::clone(&count);

    let source_count = cfg.adlist.len();
    let pb = mb.add(ProgressBar::new_spinner());
    pb.set_style(spinner_style);
    pb.enable_steady_tick(100);
    pb.set_draw_delta(500);

    thread::spawn(move || {
        let locale = SystemLocale::default().unwrap();
        while let Ok(line) = rx.recv() {
            count_c.fetch_add(1, Ordering::Relaxed);
            pb.inc(1);

            for output in &mut outputs {
                output.write_host(&line).expect("failed to write host into output");
            }
        }

        for output in &mut outputs {
            output.finalise().expect("failed to finalise output");
        }

        let count = count_c.load(Ordering::Relaxed).to_formatted_string(&locale);
        pb.println(&format!("INFO Read {} domains from {} source(s)", count, source_count,));
        pb.finish_and_clear();
    });

    for adlist in &cfg.adlist {
        let tx = tx.clone();
        let adlist = adlist.clone();
        let timeout = opt.timeout;

        let pb = mb.add(ProgressBar::new(0));
        pb.set_style(download_style.clone());

        thread::spawn(move || {
            match adlist.read(timeout) {
                Ok((len, reader)) => {
                    pb.println(format!("INFO Reading adlist from {}...", adlist.source));
                    pb.set_length(len);
                    let reader = pb.wrap_read(reader);
                    let reader = BufReader::new(reader);

                    for (line_idx, line) in reader.lines().enumerate() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_e) => continue,
                        };

                        if line.starts_with('#') || line.is_empty() {
                            continue;
                        }

                        let parsed_line = match adlist.format {
                            AdlistFormat::Hosts => parse_hosts_line(line.trim()),
                            AdlistFormat::Domains => parse_domains_line(line.trim()),
                        };

                        if let Some(parsed_line) = parsed_line {
                            if parsed_line.is_empty() || parsed_line == "." {
                                pb.println(format!(
                                    "WARN While reading {}, line #{} (\"{}\") was parsed into an all-matching entry, \
                                     so it was ignored",
                                    adlist.source,
                                    line_idx + 1,
                                    line
                                ));
                                continue;
                            }

                            tx.send(parsed_line).expect("failed to send parsed line");
                        }
                    }
                }
                Err(e) => warn!("Reading adlist from {} failed: {}", adlist.source, e),
            };
            pb.finish_and_clear();
        });
    }

    // when the requester threads finish, they drop their clones of the channel tx. the output writer thread ends when
    // all the tx's have been dropped. drop ours now since we don't need it
    drop(tx);
    mb.join_and_clear().unwrap();
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
    Ok(match &opt.config {
        Some(path) => confy::load_path(path)?,
        None => confy::load(APP_NAME)?,
    })
}

fn parse_hosts_line(line: &str) -> Option<String> {
    if let Some((address, host)) = split_once(&line, " ") {
        let address: IpAddr = address.parse().ok()?;

        // assumes the address in the host mapping is the 'unspecified' address 0.0.0.0
        if address.is_unspecified() {
            // disallow having an IP address as the host
            if host.parse::<IpAddr>().is_err() {
                return Some(host.trim().to_string());
            }
        }
    }

    None
}

fn parse_domains_line(line: &str) -> Option<String> {
    Some(line.to_owned())
}

// TODO: replace with https://doc.rust-lang.org/nightly/std/primitive.str.html#method.split_once once stabilised
fn split_once<'a>(s: &'a str, separator: &str) -> Option<(&'a str, &'a str)> {
    let mut split = s.split(separator);
    let first = split.next();
    let second = split.next();

    if let Some(first) = first {
        if let Some(second) = second {
            return Some((first, second));
        }
    }

    None
}
