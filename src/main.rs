#![feature(str_split_once)]

mod logging;

use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::IpAddr,
    path::PathBuf,
};

use log::*;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use url::Url;

const APP_NAME: &str = "pdns-singularity";
const DEFAULT_OUTPUT: &str = "/etc/pdns/blackhole-hosts";

#[derive(Debug, StructOpt)]
#[structopt(
    name = APP_NAME,
    about = "Gathers blacklisted DNS domains into a PDNS Recursor hosts-file."
)]
struct Opt {
    /// Enable verbose logging
    #[structopt(short, long)]
    verbose: bool,
    /// Custom path to the app's configuration file.
    #[structopt(short, long)]
    config: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    adlists: Vec<Adlist>,
    output: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct Adlist {
    source: Url,
    format: AdlistFormat,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AdlistFormat {
    Hosts,
    Domains,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            adlists: Default::default(),
            output: PathBuf::from(DEFAULT_OUTPUT),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    setup_logging(&opt)?;
    let cfg = load_config(&opt)?;

    debug!("{:?}", opt);
    debug!("{:?}", cfg);

    info!("Writing hosts into {}", cfg.output.display());
    let mut output = File::create(&cfg.output)?;

    let mut total = 0;
    for adlist in &cfg.adlists {
        if let Some(reader) = adlist.get_reader() {
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
                    writeln!(&mut output, "{}", line)?;
                    total += 1;
                    count += 1;
                }
            }

            info!("Read adlist with {} hosts", count);
        }
    }

    info!("Read {} hosts in total from {} source(s)", total, cfg.adlists.len());
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

impl Adlist {
    fn get_reader(&self) -> Option<BufReader<Box<dyn Read>>> {
        match self.source.scheme() {
            "http" | "https" => {
                info!("Requesting adlist from {}...", self.source);

                // TODO: add configurable timeouts
                let resp = ureq::get(self.source.as_str()).timeout_connect(1_000).call();
                debug!("Got response status {}", resp.status());

                if resp.ok() {
                    Some(BufReader::new(Box::new(resp.into_reader()) as Box<dyn Read>))
                } else {
                    error!(
                        "Requesting adlist failed. Got response status {}. Response body:\n{}",
                        resp.status(),
                        resp.into_string()
                            .expect("failed to turn error response body into string")
                    );
                    None
                }
            }
            "file" => {
                let path = match self.source.to_file_path() {
                    Ok(path) => path,
                    Err(()) => {
                        error!("Invalid path for file scheme: {}", self.source);
                        return None;
                    }
                };
                info!("Reading adlist from {}...", path.display());

                let file = match File::open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        error!("Failed to open adlist file: {}", e);
                        return None;
                    }
                };
                Some(BufReader::new(Box::new(file) as Box<dyn Read>))
            }
            scheme => {
                error!("Unsupported adlist source scheme: '{}'", scheme);
                None
            }
        }
    }
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
