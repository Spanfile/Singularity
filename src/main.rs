#![warn(clippy::if_not_else)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::non_ascii_literal)]
#![warn(clippy::panic_in_result_fn)]
#![warn(clippy::clippy::too_many_lines)]
#![warn(clippy::clippy::single_match_else)]

mod config;
mod error;
mod logging;
mod output;

use anyhow::Context;
use config::{Adlist, AdlistFormat, Config};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use io::{BufRead, BufReader};
use log::*;
use mpsc::Receiver;
use num_format::{SystemLocale, ToFormattedString};
use output::Output;
use std::{
    collections::HashSet,
    fmt::Display,
    io,
    net::IpAddr,
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, SyncSender},
        Arc,
    },
    thread,
};
use structopt::StructOpt;

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const HTTP_CONNECT_TIMEOUT: u64 = 30_000;

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
    let (tx, rx) = mpsc::sync_channel::<String>(1024);
    spawn_writer_thread(&mb, rx, outputs, cfg.adlist.len());

    // move the whitelist out of the config (partial move) and stick it into an Arc, so it can be shared between all
    // reader threads
    let whitelist = Arc::new(cfg.whitelist);
    // move each adlist out of the config (also a partial move)
    for adlist in cfg.adlist {
        let tx = tx.clone();
        spawn_reader_thread(&mb, adlist, tx, opt.timeout, Arc::clone(&whitelist));
    }

    // when the requester threads finish, they drop their clones of the channel tx. the output writer thread ends when
    // all the tx's have been dropped. drop ours now since we don't need it
    drop(tx);
    mb.join_and_clear().unwrap();
    Ok(())
}

fn spawn_writer_thread(mb: &MultiProgress, rx: Receiver<String>, mut outputs: Vec<Output>, source_count: usize) {
    let spinner_style = ProgressStyle::default_spinner().template("{spinner} {pos} domains read so far...");
    let pb = mb.add(ProgressBar::new_spinner());
    pb.set_style(spinner_style);
    pb.enable_steady_tick(100);
    pb.set_draw_delta(500);

    let count = Arc::new(AtomicUsize::new(0));
    thread::spawn(move || {
        let locale = SystemLocale::default().unwrap();
        while let Ok(line) = rx.recv() {
            count.fetch_add(1, Ordering::Relaxed);
            pb.inc(1);

            for output in &mut outputs {
                output.write_host(&line).expect("failed to write host into output");
            }
        }

        for output in outputs {
            output.finalise().expect("failed to finalise output");
        }

        let count = count.load(Ordering::Relaxed).to_formatted_string(&locale);
        pb.println(&format!("INFO Read {} domains from {} source(s)", count, source_count));
        pb.finish_and_clear();
    });
}

fn spawn_reader_thread(
    mb: &MultiProgress,
    adlist: Adlist,
    tx: SyncSender<String>,
    timeout: ConnectTimeout,
    whitelist: Arc<HashSet<String>>,
) {
    let download_style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40}] {bytes}/{total_bytes} ({bytes_per_sec})")
        .progress_chars("=> ");
    let pb = mb.add(ProgressBar::new(0));
    pb.set_style(download_style);

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
                        AdlistFormat::Domains => Some(line.trim().to_owned()),
                    };

                    if let Some(parsed_line) = parsed_line {
                        if parsed_line.is_empty() || parsed_line == "." {
                            pb.println(format!(
                                "WARN While reading {}, line #{} (\"{}\") was parsed into an all-matching entry, so \
                                 it was ignored",
                                adlist.source,
                                line_idx + 1,
                                line
                            ));
                            continue;
                        }

                        if whitelist.contains(&parsed_line) {
                            pb.println(format!("INFO Ignoring whitelisted entry '{}'", parsed_line));
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
