#![feature(str_split_once)]
mod config;
mod logging;
mod output;

use anyhow::Context;
use config::{AdlistFormat, Config};
use io::BufRead;
use log::*;
use output::Output;
use std::{fmt::Display, io, net::IpAddr, path::PathBuf, str::FromStr};
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

    let mut outputs = Vec::new();
    for output_cfg in &cfg.output {
        let mut output = Output::from_config(output_cfg).with_context(|| "Failed to create output")?;
        output.write_primer()?;
        outputs.push(output);
    }

    let mut total = 0;
    for adlist in &cfg.adlist {
        if let Some(reader) = adlist.get_reader(opt.timeout) {
            let mut count = 0;
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        warn!("Invalid line in output. {}", e);
                        continue;
                    }
                };

                let line = match adlist.format {
                    AdlistFormat::Hosts => parse_hosts_line(line),
                    AdlistFormat::Domains => parse_domains_line(line),
                };

                if let Some(line) = line {
                    for output in &mut outputs {
                        output.write_host(&line)?;
                    }

                    total += 1;
                    count += 1;
                }
            }

            info!("Read adlist with {} hosts", count);
        }
    }

    for output in &mut outputs {
        output.finalise()?;
    }

    info!(
        "Read {} blackholed hosts in total from {} source(s)",
        total,
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

fn parse_hosts_line(line: String) -> Option<String> {
    if !line.starts_with('#') {
        if let Some((address, host)) = line.split_once(" ") {
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

fn parse_domains_line(line: String) -> Option<String> {
    Some(line)
}
