#![warn(clippy::missing_errors_doc)]
#![warn(clippy::missing_panics_doc)]
#![warn(missing_docs)]

//! A library for pulling known malicious domains into one or more blackhole lists in various formats.
//!
//! This documentation is for the Singularity library, for the Singularity CLI executable see TODO.
//!
//! # Usage
//!
//! To use the library in your program, add it as a dependency and disable all default features. You may then enable
//! additional features described in [features](#features). Since both the Singularity library and the Singularity CLI
//! program are both in the same crate, the additional dependencies for the CLI program are behind the `bin`-feature,
//! which is enabled by default. These dependencies are not needed to use the library, so you should disable the
//! feature.
//!
//! ```toml
//! singularity = { version = "0.9.0", default-features=false }
//! ```
//!
//! # Example
//!
//! ```
//! // Create a new Singularity builder
//! let mut builder = Singularity::builder();
//!
//! // Add one or more adlists. Adlists are sources of malicious domains. See the Adlist struct's documentation for more
//! // information.
//! builder.add_adlist(
//!     "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
//!     // The source is formatted as a normal hosts-file: specify that format here. Other supported formats are documented
//!     // in the AdlistFormat enum.
//!     AdlistFormat::Hosts,
//! );
//!
//! // Add one or more outputs. Outputs are files in the filesystem all the domains from the sources are written to in a
//! // certain format. See the OutputBuilder's documentation for more information.
//! builder.add_output(
//!     // Create a new Output builder and set its type and filesystem destination.
//!     Output::builder(OutputType::PdnsLua {
//!             output_metric: true,
//!             metric_name: "blocked-queries"
//!         }, "/etc/pdns/blackhole.lua")
//!         // Use a different blackhole address from the default.
//!         .blackhole_address("0.0.0.0")
//!         // Deduplicate entries in the output.
//!         .deduplicate(true)
//!         // Finalise the builder to get a complete Output. Building the Output may fail; see the OutputBuilder
//!         // documentation for more information.
//!         .build()?;
//! );
//!
//! // Whitelist a certain domain to prevent it from being blackholed even if present in the sources.
//! builder.whitelist_domain("example.com");
//!
//! // Finalise the builder to get a complete Singularity object.
//! let singularity = builder.build();
//!
//! // Run Singularity. It'll read all the sources for their domains and write them to the configured outputs in their
//! // corresponding formats. The function will return once the process is finished.
//! singularity.run()?;
//! ```
//!
//! # Progress reporting
//!
//! By default Singularity will not output anything while running, and returns only once its finished running or if an
//! error occurs. You can however give it a progress callback function it'll call during operation to report on its
//! progress and status. This callback will be called simultaneously by multiple threads so it has to be thread-safe.
//! Anything it borrows has to live as long as the running [`Singularity`] object.
//!
//! The Singularity CLI program uses this callback to render progress bars on the terminal.
//!
//! ```
//! singularity
//!     .progress_callback(|progress| {
//!         // The progress parameter is a Progress enum that contains information about what Singularity is doing
//!     })
//!     .run()?;
//! ```
//!
//! ## Example: count how many domains have been read from all the sources
//!
//! ```
//! let count = AtomicUsize::new(0);
//!
//! singularity
//!     .progress_callback(|progress| {
//!         if let Progress::DomainWritten(_domain) = progress {
//!             count.fetch_add(1, Ordering::Relaxed);
//!         }
//!     })
//!     .run()?;
//! ```
//!
//! # Runtime
//!
//! When Singularity runs, it begins by "activating" each output. This means it'll create a temporary file to write the
//! output file into, and writing a "primer" to that file so any blackholed domains may then be written to the file in
//! sequence. The activation will fail if either the temporary file creation or writing the primer fails. This error is
//! returned immediately from the [`run()`](Singularity::run)-method.
//!
//! Singularity then spawns a single thread responsible for writing domains to each output, and a thread for each adlist
//! responsible for reading that adlist. The reader threads go through their source line by line and attempt to parse
//! them as domains in their given format. They then emit their read domains to the writer thread. Any errors in these
//! threads are handled gracefully and propagated to the user via the [progress callback](#progress-reporting). If any
//! of the reader threads panic, Singularity will continue operating without that reader. If the writer thread panics,
//! Singularity will abort the process. When the writer thread and all the reader threads exit succesfully, Singularity
//! will return a success.
//!
//! # Features
//!
//! The library supports these additional features:
//!
//! - `serde`: Enable [serde] serialization and deserialization for the [`Adlist`] and [`Output`] types.
//! - `bin`: Enable additional dependencies required to build the Singularity CLI binary. This feature is never needed
//!   when using only the library, and the binary already depends on the library with this flag enabled.
//!
//! [serde]: https://serde.rs/

mod adlist;
mod builder;
mod error;
mod output;
mod progress_read;

pub use adlist::{Adlist, AdlistFormat};
pub use builder::SingularityBuilder;
pub use error::{Result, SingularityError};
pub use output::{
    Output, OutputBuilder, OutputType, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_BLACKHOLE_ADDRESS_V6, DEFAULT_DEDUPLICATE,
    DEFAULT_METRIC_NAME, DEFAULT_OUTPUT_METRIC,
};

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
