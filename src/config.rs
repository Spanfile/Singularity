use crate::{error::SingularityError, ConnectTimeout};
use log::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::File, io::Read, net::IpAddr, path::PathBuf, time::Duration};
use ureq::Error;
use url::Url;

const DEFAULT_BLACKHOLE_ADDRESS: &str = "0.0.0.0";
const HTTP_READ_TIMEOUT: u64 = 10_000;

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default)]
    pub whitelist: HashSet<String>,
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
    Hosts {
        #[serde(default)]
        include: Vec<PathBuf>,
    },
    #[serde(rename = "pdns-lua")]
    PdnsLua {
        #[serde(default = "default_output_metric")]
        output_metric: bool,
        #[serde(default = "default_metric_name")]
        metric_name: String,
    },
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

fn default_output_metric() -> bool {
    true
}

fn default_metric_name() -> String {
    String::from("blocked-queries")
}

impl Adlist {
    pub(crate) fn read(&self, connect_timeout: ConnectTimeout) -> anyhow::Result<(u64, Box<dyn Read>)> {
        match self.source.scheme() {
            "http" | "https" => {
                let agent = ureq::AgentBuilder::new()
                    .timeout_connect(Duration::from_millis(connect_timeout.0))
                    .timeout_read(Duration::from_millis(HTTP_READ_TIMEOUT))
                    .build();
                let resp: ureq::Response = match agent.get(self.source.as_str()).call() {
                    Ok(resp) => resp,
                    Err(Error::Status(code, resp)) => {
                        return Err(SingularityError::RequestFailed(code, resp.into_string()?).into())
                    }
                    Err(e) => return Err(e.into()),
                };
                let len: u64 = resp
                    .header("Content-Length")
                    .ok_or(SingularityError::MissingContentLengthHeader)?
                    .parse()?;
                debug!("Got response status {} with len {}", resp.status(), len);
                Ok((len, Box::new(resp.into_reader()) as Box<dyn Read>))
            }
            "file" => {
                let path = match self.source.to_file_path() {
                    Ok(path) => path,
                    Err(()) => {
                        return Err(SingularityError::InvalidFilePath(self.source.as_str().to_string()).into());
                    }
                };

                let file = File::open(&path)?;
                let meta = file.metadata()?;
                Ok((meta.len(), Box::new(file) as Box<dyn Read>))
            }
            scheme => Err(SingularityError::UnsupportedUrlScheme(scheme.to_string()).into()),
        }
    }
}
