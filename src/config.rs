use crate::{error::SingularityError, ConnectTimeout};
use log::*;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, net::IpAddr, path::PathBuf};
use url::Url;

const DEFAULT_BLACKHOLE_ADDRESS: &str = "0.0.0.0";
const HTTP_READ_TIMEOUT: u64 = 1_000;

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    pub adlist: Vec<Adlist>,
    pub output: Vec<OutputConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OutputConfig {
    #[serde(flatten)]
    pub ty: OutputConfigType,
    pub destination: PathBuf,
    #[serde(default = "default_blackhole_address")]
    pub blackhole_address: IpAddr,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum OutputConfigType {
    #[serde(rename = "hosts")]
    Hosts { include: Vec<PathBuf> },
    #[serde(rename = "pdns-lua")]
    PdnsLua,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Adlist {
    pub source: Url,
    #[serde(default)]
    pub format: AdlistFormat,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub(crate) fn read(&self, connect_timeout: ConnectTimeout) -> anyhow::Result<(u64, Box<dyn Read>)> {
        match self.source.scheme() {
            "http" | "https" => {
                info!("Requesting adlist from {}...", self.source);

                let resp = ureq::get(self.source.as_str())
                    .timeout_connect(connect_timeout.0)
                    .timeout_read(HTTP_READ_TIMEOUT)
                    .call();
                let len: u64 = resp
                    .header("Content-Length")
                    .ok_or(SingularityError::MissingContentLengthHeader)?
                    .parse()?;
                debug!("Got response status {} with len {}", resp.status(), len);

                if resp.ok() {
                    Ok((len, Box::new(resp.into_reader()) as Box<dyn Read>))
                } else {
                    Err(SingularityError::RequestFailed(resp.status(), resp.into_string()?).into())
                }
            }
            "file" => {
                let path = match self.source.to_file_path() {
                    Ok(path) => path,
                    Err(()) => {
                        return Err(SingularityError::InvalidFilePath(self.source.as_str().to_string()).into());
                    }
                };
                info!("Reading adlist from {}...", path.display());

                let file = File::open(&path)?;
                let meta = file.metadata()?;
                Ok((meta.len(), Box::new(file) as Box<dyn Read>))
            }
            scheme => Err(SingularityError::UnsupportedUrlScheme(scheme.to_string()).into()),
        }
    }
}
