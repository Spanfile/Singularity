use crate::{error::SingularityError, Result};
use std::{fs::File, io::Read, time::Duration};
use url::Url;

const HTTP_READ_TIMEOUT: u64 = 10_000;

/// Represents a source for a list that contains various domains in a certain format.
#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Adlist {
    pub(crate) source: Url,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) format: AdlistFormat,
}

/// The different kinds of formats supported for adlists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "lowercase") // TODO: turn this rename to just aliases for the fields
)]
pub enum AdlistFormat {
    /// Hosts-file formatting. Each line in the source is in the same format as they would be in a hosts-file:
    /// ```
    /// 0.0.0.0 example.com
    /// 0.0.0.0 google.com
    /// ...
    /// ```
    /// It is assumed the address in each line is the unspecified address; `0.0.0.0` for IPv4 and `::`
    /// for IPv6. The host in each line must be a domain name; IP addresses are not allowed.
    Hosts,
    /// Each line in the source is one domain name:
    /// ```
    /// example.com
    /// google.com
    /// ...
    /// ```
    Domains,
    /// Each line is an `address`-configuration for dnsmasq:
    /// ```
    /// address=/example.com/#
    /// address=/google.com/#
    /// ...
    /// ```
    Dnsmasq,
}

impl Default for AdlistFormat {
    fn default() -> Self {
        Self::Hosts
    }
}

impl std::fmt::Display for AdlistFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdlistFormat::Hosts => write!(f, "Hosts"),
            AdlistFormat::Domains => write!(f, "Domains"),
            AdlistFormat::Dnsmasq => write!(f, "Dnsmasq"),
        }
    }
}

impl Adlist {
    /// Returns a new adlist with the given source and format. The given source string will be parsed into an URL. If
    /// you wish to supply an already constructed URL, please use the [with_url_source](Adlist::with_url_source)
    /// method.
    ///
    /// # Errors
    ///
    /// Will return [`SingularityError::Url`](SingularityError::Url) if the given source string fails to be parsed into
    /// an URL.
    pub fn new<S>(source: S, format: AdlistFormat) -> Result<Self>
    where
        S: AsRef<str>,
    {
        let source = Url::parse(source.as_ref())?;
        Ok(Self { source, format })
    }

    /// Returns a new adlist with the given source and format. If you have the URL as a string, it may be more
    /// convenient to use the [new](Adlist::new) method instead that will attempt to parse the string into an URL.
    pub fn with_url_source(source: Url, format: AdlistFormat) -> Self {
        Self { source, format }
    }

    /// Returns a reference to the adlist's source URL.
    pub fn source(&self) -> &Url {
        &self.source
    }

    /// Returns the adlist's format.
    pub fn format(&self) -> AdlistFormat {
        self.format
    }

    /// Returns a tuple of the possible elength of the content, and a reader for the content.
    ///
    /// When reading from an HTTP source, the server's response may use chunk transfer encoding in which case the
    /// content cannot be determined ahead of time.
    pub(crate) fn read(&self, connect_timeout: u64) -> Result<(Option<u64>, Box<dyn Read>)> {
        match self.source.scheme() {
            "http" | "https" => {
                let agent = ureq::AgentBuilder::new()
                    .timeout_connect(Duration::from_millis(connect_timeout))
                    .timeout_read(Duration::from_millis(HTTP_READ_TIMEOUT))
                    .build();

                let resp: ureq::Response = match agent.get(self.source.as_str()).call() {
                    Ok(resp) => resp,
                    Err(ureq::Error::Status(code, resp)) => {
                        return Err(SingularityError::RequestFailed(code, resp.into_string()?))
                    }
                    Err(e) => return Err(e.into()),
                };

                // the header names may or may not be lowercased
                let len = resp
                    .header("Content-Length")
                    .or_else(|| resp.header("content-length"))
                    .map(str::parse::<u64>)
                    .transpose()
                    .map_err(|e| {
                        SingularityError::InvalidResponse(format!(
                            "invalid content-length header (not an integer): {}",
                            e
                        ))
                    })?;

                Ok((len, Box::new(resp.into_reader()) as Box<dyn Read>))
            }
            "file" => {
                let path = match self.source.to_file_path() {
                    Ok(path) => path,
                    Err(()) => {
                        return Err(SingularityError::InvalidFilePath(self.source.as_str().to_string()));
                    }
                };

                let file = File::open(&path)?;
                let meta = file.metadata()?;
                Ok((Some(meta.len()), Box::new(file) as Box<dyn Read>))
            }
            scheme => Err(SingularityError::UnsupportedUrlScheme(scheme.to_string())),
        }
    }
}
