mod adlist;
mod builder;
mod error;
mod output;

pub use adlist::Adlist;
pub use error::{Result, SingularityError};
pub use output::{Output, OutputConfig, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_BLACKHOLE_ADDRESS_V6};

use adlist::AdlistFormat;
use builder::SingularityBuilder;
use lazy_static::lazy_static;
use output::ActiveOutput;
use regex::Regex;
use std::{
    collections::HashSet,
    io::{BufRead, BufReader},
    net::IpAddr,
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread::{self, JoinHandle},
};

pub const HTTP_CONNECT_TIMEOUT: u64 = 30_000;

pub struct Singularity {
    adlists: Vec<Adlist>,
    outputs: Vec<Output>,
    whitelist: HashSet<String>,
    http_timeout: u64,
    prog_callback: Option<Box<dyn Fn(Progress)>>,
}

#[derive(Debug)]
pub struct Progress {}

impl Singularity {
    pub fn builder() -> SingularityBuilder {
        SingularityBuilder::new()
    }

    #[must_use]
    pub fn progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(Progress) + 'static,
    {
        self.prog_callback = Some(Box::new(callback));
        self
    }

    pub fn run(self) -> Result<()> {
        let mut threads = Vec::new();
        let (tx, rx) = mpsc::sync_channel::<String>(1024);
        let active_outputs = self
            .outputs
            .into_iter()
            .map(|output| output.activate())
            .collect::<Result<Vec<_>>>()?;

        threads.push(spawn_writer_thread(active_outputs, rx));

        // move the whitelist out of the config (partial move) and stick it into an Arc, so it can be shared between all
        // reader threads
        let whitelist = Arc::new(self.whitelist);
        // move each adlist out of the config (also a partial move)
        for adlist in self.adlists {
            let tx = tx.clone();
            threads.push(spawn_reader_thread(
                adlist,
                tx,
                self.http_timeout,
                Arc::clone(&whitelist),
            ));
        }

        // when the reader threads finish, they drop their clones of the channel tx. the output writer thread ends
        // when all the tx's have been dropped. drop our remaining tx now to ensure the writer thread ends succesfully
        drop(tx);

        for handle in threads {
            // TODO: handle more gracefully
            handle.join().expect("thread failed");
        }

        Ok(())
    }
}

fn spawn_writer_thread(mut active_outputs: Vec<ActiveOutput>, rx: Receiver<String>) -> JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(line) = rx.recv() {
            for output in &mut active_outputs {
                output.write_host(&line).expect("failed to write host into output");
            }
        }

        for output in active_outputs {
            output.finalise().expect("failed to finalise output");
        }
    })
}

fn spawn_reader_thread(
    adlist: Adlist,
    tx: SyncSender<String>,
    timeout: u64,
    whitelist: Arc<HashSet<String>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        match adlist.read(timeout) {
            Ok((len, reader)) => {
                // TODO: emit a progress message with the adlist length and source

                let reader = BufReader::new(reader);

                for (line_idx, line) in reader.lines().enumerate() {
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
                            // TODO: emit progress warning message

                            // pb.println(format!(
                            //     "WARN While reading {}, line #{} (\"{}\") was parsed into an all-matching entry, so \
                            //      it was ignored",
                            //     adlist.source,
                            //     line_idx + 1,
                            //     line
                            // ));
                            continue;
                        }

                        if whitelist.contains(&parsed_line) {
                            // TODO: emit progress info message

                            // pb.println(format!("INFO Ignoring whitelisted entry '{}'", parsed_line));
                            continue;
                        }

                        tx.send(parsed_line).expect("failed to send parsed line");
                    }
                }
            }
            Err(e) => {
                // TODO: emit progress warning message

                // warn!("Reading adlist from {} failed: {}", adlist.source, e);
            }
        }
    })
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
