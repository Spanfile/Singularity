#![feature(str_split_once)]

mod logging;

use std::{
    fmt::Display,
    fs::File,
    io,
    io::{BufRead, BufReader, Read, Write},
    net::IpAddr,
    path::PathBuf,
    str::FromStr,
};

use log::*;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use url::Url;

const APP_NAME: &str = "pdns-singularity";
const DEFAULT_OUTPUT: &str = "/etc/pdns/blackhole-hosts";
const HTTP_READ_TIMEOUT: u64 = 1_000;
const HTTP_CONNECT_TIMEOUT: u64 = 1_000;
const DEFAULT_BLACKHOLE_ADDRESS: &str = "0.0.0.0";

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
    about = "Gathers blacklisted DNS domains into a PDNS Recursor hosts-file."
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

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    adlists: Vec<Adlist>,
    output: PathBuf,
    #[serde(rename = "blackhole-address")]
    blackhole_address: IpAddr,
    include: Vec<PathBuf>,
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
            blackhole_address: DEFAULT_BLACKHOLE_ADDRESS
                .parse()
                .expect("failed to parse default blackhole address"),
            include: Default::default(),
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
                    writeln!(&mut output, "{}", host_blackhole(cfg.blackhole_address, &line))?;
                    total += 1;
                    count += 1;
                }
            }

            info!("Read adlist with {} hosts", count);
        }
    }

    for include in &cfg.include {
        debug!("Including extra hosts from {}", include.display());

        let mut file = File::open(include)?;
        writeln!(&mut output, "\n# extra hosts included from {}\n", include.display())?;
        io::copy(&mut file, &mut output)?;
    }

    info!(
        "Read {} blackholed hosts in total from {} source(s)",
        total,
        cfg.adlists.len()
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

impl Adlist {
    fn get_reader(&self, connect_timeout: ConnectTimeout) -> Option<BufReader<Box<dyn Read>>> {
        match self.source.scheme() {
            "http" | "https" => {
                info!("Requesting adlist from {}...", self.source);

                let resp = ureq::get(self.source.as_str())
                    .timeout_connect(connect_timeout.0)
                    .timeout_read(HTTP_READ_TIMEOUT)
                    .call();
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

fn host_blackhole(blackhole_address: IpAddr, host: &str) -> String {
    format!("{} {}", blackhole_address, host)
}
