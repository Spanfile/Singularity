use crate::error::SingularityError;
use log::*;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, time::Duration};
use url::Url;

const HTTP_READ_TIMEOUT: u64 = 10_000;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Adlist {
    pub(crate) source: Url,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) format: AdlistFormat,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize), serde(rename_all = "lowercase"))]
pub enum AdlistFormat {
    Hosts,
    Domains,
    DnsMasq,
}

impl Default for AdlistFormat {
    fn default() -> Self {
        Self::Hosts
    }
}

impl Adlist {
    /// Returns a new adlist with the given source and format.
    pub fn new(source: Url, format: AdlistFormat) -> Adlist {
        Adlist { source, format }
    }

    /// Returns a tuple of the possible elength of the content, and a reader for the content.
    ///
    /// When reading from an HTTP source, the server's response may use chunk transfer encoding in which case the
    /// content cannot be determined ahead of time.
    pub(crate) fn read(&self, connect_timeout: u64) -> anyhow::Result<(Option<u64>, Box<dyn Read>)> {
        match self.source.scheme() {
            "http" | "https" => {
                let agent = ureq::AgentBuilder::new()
                    .timeout_connect(Duration::from_millis(connect_timeout))
                    .timeout_read(Duration::from_millis(HTTP_READ_TIMEOUT))
                    .build();

                let resp: ureq::Response = match agent.get(self.source.as_str()).call() {
                    Ok(resp) => resp,
                    Err(ureq::Error::Status(code, resp)) => {
                        return Err(SingularityError::RequestFailed(code, resp.into_string()?).into())
                    }
                    Err(e) => return Err(e.into()),
                };

                // the header names may or may not be lowercased
                let len = resp
                    .header("Content-Length")
                    .or_else(|| resp.header("content-length"))
                    .map(str::parse::<u64>)
                    .transpose()?;

                if let Some(len) = len {
                    debug!("Got response status {} with length {}", resp.status(), len);
                } else {
                    debug!("Got response status {} with indeterminate length", resp.status());
                }

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
                Ok((Some(meta.len()), Box::new(file) as Box<dyn Read>))
            }
            scheme => Err(SingularityError::UnsupportedUrlScheme(scheme.to_string()).into()),
        }
    }
}
