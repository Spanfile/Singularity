pub(crate) mod adlist;
pub(crate) mod builder;
pub(crate) mod output;

use self::{
    adlist::{Adlist, AdlistFormat},
    builder::SingularityBuilder,
    output::{ActiveOutput, Output},
};
use crate::{progress_read::ProgressRead, Result, SingularityError};
use crossbeam_utils::{atomic::AtomicCell, thread};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashSet,
    io::{BufRead, BufReader},
    net::IpAddr,
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
};

/// The default timeout to wait for HTTP connects to succeed in milliseconds: `30 000` (30 seconds).
pub const HTTP_CONNECT_TIMEOUT: u64 = 30_000;

type ProgressCallback<'a> = Box<dyn Fn(Progress) + Send + Sync + 'a>;

/// The Singularity runner.
///
/// See the crate-level documentation for details on usage.
pub struct Singularity<'a> {
    adlists: Vec<Adlist>,
    outputs: Vec<Output>,
    whitelist: HashSet<String>,
    http_timeout: u64,
    prog_callback: ProgressCallback<'a>,
}

/// The progress report enum Singularity emits during operation.
#[derive(Debug)]
pub enum Progress<'a> {
    /// Begin reading an adlist.
    BeginAdlistRead {
        /// The adlist's source URL.
        source: &'a str,
        /// The adlists's length, if determined. Some HTTP/HTTPS sources return the content using chunk transfer
        /// encoding, which means their length cannot be determined ahead of time.
        length: Option<u64>,
    },
    /// Progress reading through an adlist source.
    ReadProgress {
        /// The adlist's source URL.
        source: &'a str,
        /// How many bytes have been read so far from the source.
        bytes: u64,
        /// How many more bytes have been read since the last progress report for this source.
        delta: u64,
    },
    /// An adlist finished reading.
    FinishAdlistRead {
        /// The adlists's source URL.
        source: &'a str,
    },
    /// Reading an adlist failed.
    ReadingAdlistFailed {
        /// The adlist's source URL.
        source: &'a str,
        /// The reason the reading failed.
        reason: Box<SingularityError>,
    },
    /// A domain was read succesfully from an adlist and was written to all outputs.
    DomainWritten(
        /// The domain that was written.
        &'a str,
    ),
    /// A domain was read succesfully from an adlist but it was ignored because it is whitelisted.
    WhitelistedDomainIgnored {
        /// The adlists's source URL this domain originated from.
        source: &'a str,
        /// The domain that was ignored.
        domain: &'a str,
    },
    /// A domain was read succesfully but it parsed into an all-matching entry so it was ignored.
    AllMatchingLineIgnored {
        /// The adlist's source URL this domain originated from.
        source: &'a str,
        /// The line number for this domain in the adlist.
        line_number: usize,
        /// The line that was parsed from the adlist.
        line: &'a str,
    },
}

impl<'a> Singularity<'a> {
    /// Returns a new [`SingularityBuilder`].
    pub fn builder() -> SingularityBuilder {
        SingularityBuilder::new()
    }

    /// Set the progress callback that Singularity calls during operation to report on its progress and status.
    #[must_use]
    pub fn progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(Progress) + Send + Sync + 'a,
    {
        self.prog_callback = Box::new(callback);
        self
    }

    /// Runs Singularity. See the crate-level documentation for details on the runtime characteristics.
    ///
    /// This will consume the [`Singularity`] object.
    ///
    /// # Errors
    ///
    /// This method will return an error if activating the configured outputs fails. See the crate-level documentation
    /// for details on how it might fail.
    ///
    /// # Panics
    ///
    /// This method will panic if spawning the writer/reader threads fail.
    pub fn run(self) -> Result<()> {
        let (tx, rx) = mpsc::sync_channel::<String>(1024);
        let active_outputs = self
            .outputs
            .into_iter()
            .map(|output| output.activate())
            .collect::<Result<Vec<_>>>()?;

        let cb = Arc::new(self.prog_callback);
        let whitelist = Arc::new(self.whitelist);

        thread::scope(move |s| {
            let writer_cb = Arc::clone(&cb);
            s.spawn(|_| writer_thread(active_outputs, rx, writer_cb));

            for adlist in self.adlists {
                let tx = tx.clone();
                let reader_cb = Arc::clone(&cb);
                let http_timeout = self.http_timeout;
                let reader_whitelist = Arc::clone(&whitelist);

                s.spawn(move |_| reader_thread(adlist, tx, http_timeout, reader_whitelist, reader_cb));
            }

            // TODO: abort everything if the writer thread dies
        })
        .expect("reader/writer scope failed");

        Ok(())
    }
}

fn writer_thread(mut active_outputs: Vec<ActiveOutput>, rx: Receiver<String>, cb: Arc<ProgressCallback>) {
    // TODO: handle errors here gracefully with the callback

    while let Ok(line) = rx.recv() {
        cb(Progress::DomainWritten(&line));

        for output in &mut active_outputs {
            output.write_host(&line).expect("failed to write host into output");
        }
    }

    for output in active_outputs {
        output.finalise().expect("failed to finalise output");
    }
}

fn reader_thread(
    adlist: Adlist,
    tx: SyncSender<String>,
    timeout: u64,
    whitelist: Arc<HashSet<String>>,
    cb: Arc<ProgressCallback>,
) {
    let source = adlist.source.as_str();

    match adlist.read(timeout) {
        Ok((length, reader)) => {
            cb(Progress::BeginAdlistRead { source, length });

            let read_amount = AtomicCell::<u64>::new(0);
            let last_read_amount = AtomicCell::<u64>::new(0);
            let reader = ProgressRead::new(reader, &read_amount);
            let reader = BufReader::new(reader);

            for (line_idx, line) in reader.lines().enumerate() {
                let bytes = read_amount.load();
                let delta = bytes - last_read_amount.swap(bytes);

                cb(Progress::ReadProgress { source, bytes, delta });

                let line = match line {
                    Ok(l) => l,
                    Err(_e) => continue,
                };

                if line.starts_with('#') || line.is_empty() {
                    continue;
                }

                let parsed_line = match adlist.format {
                    AdlistFormat::Hosts => parse_hosts_line(line.trim()),
                    AdlistFormat::Dnsmasq => parse_dnsmasq_line(line.trim()),
                    AdlistFormat::Domains => Some(line.trim().to_owned()),
                };

                if let Some(parsed_line) = parsed_line {
                    if parsed_line.is_empty() || parsed_line == "." {
                        cb(Progress::AllMatchingLineIgnored {
                            source: adlist.source.as_str(),
                            line_number: line_idx + 1,
                            line: &line,
                        });

                        continue;
                    }

                    if whitelist.contains(&parsed_line) {
                        cb(Progress::WhitelistedDomainIgnored {
                            source: adlist.source.as_str(),
                            domain: &parsed_line,
                        });

                        continue;
                    }

                    tx.send(parsed_line).expect("failed to send parsed line");
                }
            }

            cb(Progress::FinishAdlistRead {
                source: adlist.source.as_str(),
            });
        }
        Err(e) => cb(Progress::ReadingAdlistFailed {
            source: adlist.source.as_str(),
            reason: Box::new(e),
        }),
    }
}

fn parse_hosts_line(line: &str) -> Option<String> {
    if let Some((address, host)) = line.split_once(' ') {
        let address: IpAddr = address.parse().ok()?;

        // assumes the address in the host mapping is the 'unspecified' address, 0.0.0.0 for v4 and :: for v6
        if address.is_unspecified() {
            // disallow having an IP address as the host
            if host.parse::<IpAddr>().is_err() {
                return Some(host.trim().to_string());
            }
        }
    }

    None
}

fn parse_dnsmasq_line(line: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"(?:address|server)=/(.*)/.*"#).unwrap();
    }

    let cap = RE.captures(line)?;
    Some(String::from(&cap[1]))
}

fn noop_callback(_: Progress) {}

#[cfg(test)]
mod tests {
    #[test]
    fn hosts_parse_valid_line() {
        let valid_v4 = super::parse_hosts_line("0.0.0.0 example.com");
        assert!(matches!(valid_v4.as_deref(), Some("example.com")));

        let valid_v6 = super::parse_hosts_line(":: example.com");
        assert!(matches!(valid_v6.as_deref(), Some("example.com")));
    }

    #[test]
    fn hosts_malformed_line() {
        let invalid = super::parse_hosts_line("0.0.0.0");
        assert!(invalid.is_none());
    }

    #[test]
    fn hosts_address_as_host() {
        let invalid = super::parse_hosts_line("0.0.0.0 0.0.0.0");
        assert!(invalid.is_none());
    }

    #[test]
    fn hosts_invalid_ip_address() {
        let invalid = super::parse_hosts_line("999.0.0.0 example.com");
        assert!(invalid.is_none());
    }

    #[test]
    fn dnsmasq_parse_valid_line() {
        let valid_address = super::parse_dnsmasq_line("address=/example.com/#");
        let valid_server = super::parse_dnsmasq_line("server=/example.com/#");

        assert!(matches!(valid_address.as_deref(), Some("example.com")));
        assert!(matches!(valid_server.as_deref(), Some("example.com")));
    }

    #[test]
    fn dnsmasq_no_capture() {
        let invalid = super::parse_dnsmasq_line("address=");
        assert!(invalid.is_none());
    }
}
