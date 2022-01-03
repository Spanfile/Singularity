mod adlist;
mod builder;
mod error;
mod output;
mod progress_read;

pub use adlist::Adlist;
pub use error::{Result, SingularityError};
pub use output::{Output, OutputConfig, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_BLACKHOLE_ADDRESS_V6};

use adlist::AdlistFormat;
use builder::SingularityBuilder;
use crossbeam_utils::{atomic::AtomicCell, thread};
use lazy_static::lazy_static;
use output::ActiveOutput;
use progress_read::ProgressRead;
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

pub const HTTP_CONNECT_TIMEOUT: u64 = 30_000;

type ProgressCallback<'a> = Box<dyn Fn(Progress) + Send + Sync + 'a>;

pub struct Singularity<'a> {
    adlists: Vec<Adlist>,
    outputs: Vec<Output>,
    whitelist: HashSet<String>,
    http_timeout: u64,
    prog_callback: ProgressCallback<'a>,
}

#[derive(Debug)]
pub enum Progress<'a> {
    BeginAdlistRead {
        source: &'a str,
        length: Option<u64>,
    },
    ReadProgress {
        source: &'a str,
        bytes: u64,
        delta: u64,
    },
    FinishAdlistRead {
        source: &'a str,
    },
    ReadingAdlistFailed {
        source: &'a str,
        reason: Box<SingularityError>,
    },
    DomainWritten(&'a str),
    WhitelistedDomainIgnored {
        source: &'a str,
        domain: &'a str,
    },
    AllMatchingLineIgnored {
        source: &'a str,
        line_number: usize,
        line: &'a str,
    },
}

impl<'a> Singularity<'a> {
    pub fn builder() -> SingularityBuilder {
        SingularityBuilder::new()
    }

    #[must_use]
    pub fn progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(Progress) + Send + Sync + 'a,
    {
        self.prog_callback = Box::new(callback);
        self
    }

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
        })
        .expect("reader/writer scope failed");

        Ok(())
    }
}

fn writer_thread(mut active_outputs: Vec<ActiveOutput>, rx: Receiver<String>, cb: Arc<ProgressCallback>) {
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
                    AdlistFormat::DnsMasq => parse_dnsmasq_line(line.trim()),
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

        // assumes the address in the host mapping is the 'unspecified' address 0.0.0.0
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
