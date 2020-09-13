use crate::ConnectTimeout;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, Read},
    net::IpAddr,
    path::PathBuf,
};
use url::Url;

const DEFAULT_BLACKHOLE_ADDRESS: &str = "0.0.0.0";
const HTTP_READ_TIMEOUT: u64 = 1_000;

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    pub adlists: Vec<Adlist>,
    pub output: Vec<OutputConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum OutputConfig {
    Hosts {
        hosts: PathBuf,
        #[serde(default = "default_blackhole_address")]
        blackhole_address: IpAddr,
        #[serde(default)]
        include: Vec<PathBuf>,
    },
    PdnsLua {
        #[serde(rename = "pdns-lua")]
        pdns_lua: PathBuf,
        #[serde(default = "default_blackhole_address")]
        blackhole_address: IpAddr,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Adlist {
    pub source: Url,
    #[serde(default)]
    pub format: AdlistFormat,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AdlistFormat {
    Hosts,
    Domains,
}

impl Default for AdlistFormat {
    fn default() -> Self {
        Self::Hosts
    }
}

fn default_blackhole_address() -> IpAddr {
    DEFAULT_BLACKHOLE_ADDRESS
        .parse()
        .expect("failed to parse default blackhole address")
}

impl Adlist {
    pub(crate) fn get_reader(&self, connect_timeout: ConnectTimeout) -> Option<BufReader<Box<dyn Read>>> {
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
