mod config;
mod error;
mod logging;
mod output;

use anyhow::Context;
use config::{AdlistFormat, Config};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use io::{BufRead, BufReader};
use log::*;
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

const APP_NAME: &str = "singularity";
const HTTP_CONNECT_TIMEOUT: u64 = 1_000;

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
    name = APP_NAME,
    about = "Gathers blacklisted DNS domains into a hosts-file."
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
    let style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:80}] {bytes}/{total_bytes} ({bytes_per_sec})")
        .progress_chars("=> ");

    let (tx, rx) = mpsc::channel::<String>();
    let total = Arc::new(AtomicUsize::new(0));
    let total_c = Arc::clone(&total);

    let pb = mb.add(ProgressBar::new_spinner().with_style(ProgressStyle::default_spinner()));
    let _ = thread::spawn(move || {
        while let Ok(line) = rx.recv() {
            let old = total_c.fetch_add(1, Ordering::Relaxed);
            let total = old + 1;
            if total % 1000 == 0 {
                pb.set_message(&format!("{} domains read so far...", total));
            }

            for output in &mut outputs {
                output.write_host(&line).expect("failed to write host into output");
            }
        }

        for output in &mut outputs {
            output.finalise().expect("failed to finalise output");
        }
    });

    // let mut total = 0;
    for adlist in &cfg.adlist {
        let tx = tx.clone();
        let adlist = adlist.clone();
        let timeout = opt.timeout;

        let pb = mb.add(ProgressBar::new(0));
        pb.set_style(style.clone());

        let _ = thread::spawn(move || {
            match adlist.read(timeout) {
                Ok((len, reader)) => {
                    pb.set_length(len);
                    let reader = pb.wrap_read(reader);
                    let reader = BufReader::new(reader);

                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_e) => continue,
                        };

                        let line = match adlist.format {
                            AdlistFormat::Hosts => parse_hosts_line(line.trim()),
                            AdlistFormat::Domains => parse_domains_line(line.trim()),
                        };

                        if let Some(line) = line {
                            tx.send(line).expect("failed to send parsed line");
                        }
                    }

                    // info!("Got {} hosts", count);
                }
                Err(e) => warn!("Reading adlist from {} failed: {}", adlist.source, e),
            };
        });
    }

    // when the requester threads finish, they drop their clones of the channel tx. the output writer thread ends when
    // all the tx's have been dropped. drop ours now since we don't need it
    drop(tx);
    mb.join_and_clear().unwrap();

    info!(
        "Read {} domains from {} source(s)",
        total.load(Ordering::Relaxed),
        cfg.adlist.len()
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
    Ok(match &opt.config {
        Some(path) => confy::load_path(path)?,
        None => confy::load(APP_NAME)?,
    })
}

fn parse_hosts_line(line: &str) -> Option<String> {
    if !line.starts_with('#') {
        if let Some((address, host)) = split_once(&line, " ") {
            let address: IpAddr = address.parse().ok()?;

            // assumes the address in the host mapping is the 'unspecified' address 0.0.0.0
            if address.is_unspecified() {
                // disallow having an IP address as the host
                if host.parse::<IpAddr>().is_err() {
                    return Some(host.to_string());
                }
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
