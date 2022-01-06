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
//! ```no_run
//! # use singularity::{Singularity, SingularityError, Output, OutputType, Adlist, AdlistFormat};
//! # fn main() -> Result<(), SingularityError> {
//! // Create a new Singularity builder
//! let mut builder = Singularity::builder();
//!
//! // Add one or more adlists. Adlists are sources of malicious domains. See the Adlist struct's documentation for more
//! // information.
//! builder = builder.add_adlist(
//!     Adlist::new(
//!         "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
//!         // The source is formatted as a normal hosts-file: specify that format here. Other supported formats are documented
//!         // in the AdlistFormat enum.
//!         AdlistFormat::Hosts,
//!     )?
//! );
//!
//! // Add one or more outputs. Outputs are files in the filesystem all the domains from the sources are written to in a
//! // certain format. See the OutputBuilder's documentation for more information.
//! builder = builder.add_output(
//!     // Create a new Output builder and set its type and filesystem destination.
//!     Output::builder(OutputType::PdnsLua {
//!             output_metric: true,
//!             metric_name: "blocked-queries".to_string(),
//!         }, "/etc/pdns/blackhole.lua")
//!         // Use a certain blackhole address. This method attempts to parse the string into an IpAddr so it may fail.
//!         .blackhole_address("0.0.0.0")?
//!         // Deduplicate entries in the output.
//!         .deduplicate(true)
//!         // Finalise the builder to get a complete Output. Building the Output may fail; see the OutputBuilder
//!         // documentation for more information.
//!         .build()?,
//! );
//!
//! // Whitelist a certain domain to prevent it from being blackholed even if present in the sources.
//! builder = builder.whitelist_domain("example.com");
//!
//! // Finalise the builder to get a complete Singularity object.
//! let singularity = builder.build();
//!
//! // Run Singularity. It'll read all the sources for their domains and write them to the configured outputs in their
//! // corresponding formats. The function will return once the process is finished.
//! singularity.run()?;
//! # Ok(())
//! # }
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
//! ```no_run
//! # use singularity::{Singularity, SingularityError};
//! # fn main() -> Result<(), SingularityError> {
//! # let singularity = Singularity::builder().build();
//! singularity
//!     .progress_callback(|progress| {
//!         // The progress parameter is a Progress enum that contains information about what Singularity is doing
//!     })
//!     .run()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Example: count how many domains have been read from all the sources
//!
//! ```no_run
//! # use singularity::{Singularity, SingularityError, Progress};
//! # use std::sync::atomic::{AtomicUsize, Ordering};
//! # fn main() -> Result<(), SingularityError> {
//! # let singularity = Singularity::builder().build();
//! let count = AtomicUsize::new(0);
//!
//! singularity
//!     .progress_callback(|progress| {
//!         if let Progress::DomainWritten(_domain) = progress {
//!             count.fetch_add(1, Ordering::Relaxed);
//!         }
//!     })
//!     .run()?;
//! # Ok(())
//! # }
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

mod error;
mod progress_read;
mod singularity;

pub use crate::{
    error::{Result, SingularityError},
    singularity::{
        adlist::{Adlist, AdlistFormat},
        builder::SingularityBuilder,
        output::{
            Output, OutputBuilder, OutputType, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_BLACKHOLE_ADDRESS_V6,
            DEFAULT_DEDUPLICATE, DEFAULT_METRIC_NAME, DEFAULT_OUTPUT_METRIC,
        },
        Progress, Singularity, HTTP_CONNECT_TIMEOUT,
    },
};
