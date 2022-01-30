use crate::{Result, SingularityError};
use chrono::Local;
use io::SeekFrom;
use std::{
    collections::HashSet,
    fs::File,
    io,
    io::{Seek, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::{Path, PathBuf},
};
use tempfile::tempfile;

/// The default IPv4 blackhole address: `0.0.0.0`.
pub const DEFAULT_BLACKHOLE_ADDRESS_V4: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
/// The default IPv6 blackhole address: `::`.
pub const DEFAULT_BLACKHOLE_ADDRESS_V6: IpAddr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0));
/// The default value for output deduplication: `false`.
pub const DEFAULT_DEDUPLICATE: bool = false;
/// The default value for PDNS Lua script metric output: `true`.
pub const DEFAULT_OUTPUT_METRIC: bool = true;
/// The default name for PDNS Lua script metric: `"blocked-queries"`.
pub const DEFAULT_METRIC_NAME: &str = "blocked-queries";

const PDNS_LUA_PRIMER: &str = "b=newDS() b:add{";

/// An output for blackhole domains.
///
/// An output has various configurable settings:
/// - [Type](OutputType): the output's type. See the enum documentation for more details.
/// - Destination: path in the filesystem the output will write its final output file into. The file will be overwritten
///   if it already exists.
/// - Blackhole address: IP address the output will use as the blackholing address, which is the address DNS queries
///   will be responded to.
/// - Deduplication: ensure the output doesn't contain duplicate domains. This is only applicable when using multiple
///   [adlist sources](crate::Adlist), or if a single source happens to contain duplicate entries.
#[derive(Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Output {
    #[cfg_attr(feature = "serde", serde(flatten))]
    ty: OutputType,
    destination: PathBuf,
    #[cfg_attr(feature = "serde", serde(default = "default_blackhole_address"))]
    blackhole_address: IpAddr,
    #[cfg_attr(feature = "serde", serde(default = "default_deduplicate"))]
    deduplicate: bool,
}

/// An [`Output`'s](Output) type.
#[derive(Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize), serde(tag = "type"))]
pub enum OutputType {
    /// Output a hosts-file:
    /// ```ignore
    /// 0.0.0.0 example.com
    /// 0.0.0.0 google.com
    /// ...
    /// ```
    /// Additional hosts-files can be included in the output by specifying their paths in the `include` field.
    #[cfg_attr(feature = "serde", serde(alias = "hosts"))]
    Hosts {
        /// Additional hosts-files to include in the output.
        #[cfg_attr(feature = "serde", serde(default))]
        include: Vec<PathBuf>,
    },
    /// A PDNS Recursor Lua script.
    ///
    /// The output will construct a Lua script that can be used in the
    /// [`lua-dns-script`](lua-dns-script) setting for PDNS Recursor. The script contains a list of the blackholed
    /// domains and a `preresolve()` function that Recursor will call for every query it receives. The function looks
    /// up the queried domain in the blackhole list and if it is found, it'll set the query response's address to
    /// the configured blackhole address and return that response immediately. Additionally, it'll increment the
    /// configured metric by one if it is enabled. This metric may be accessed among all the other metrics Recursor
    /// outputs.
    ///
    /// [lua-dns-script]: https://docs.powerdns.com/recursor/settings.html#lua-dns-script
    #[cfg_attr(feature = "serde", serde(alias = "pdns-lua", alias = "pdns_lua"))]
    PdnsLua {
        /// Whether or not to output a metric of blocked domains.
        #[cfg_attr(feature = "serde", serde(default = "default_output_metric"))]
        output_metric: bool,
        /// The metric's name.
        #[cfg_attr(feature = "serde", serde(default = "default_metric_name"))]
        metric_name: String,
    },
}

#[derive(Debug)]
pub(crate) struct ActiveOutput {
    pub(crate) ty: OutputType,
    pub(crate) destination: PathBuf,
    pub(crate) blackhole_address: IpAddr,
    pub(crate) deduplicate: bool,
    pub(crate) download_dest: File,
    pub(crate) seen: HashSet<String>,
}

/// Builder for a new [`Output`].
#[derive(Debug)]
pub struct OutputBuilder {
    ty: OutputType,
    destination: PathBuf,
    blackhole_address: IpAddr,
    deduplicate: bool,
}

impl std::fmt::Display for OutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputType::Hosts { .. } => write!(f, "Hosts-file"),
            OutputType::PdnsLua { .. } => write!(f, "Recursor Lua script"),
        }
    }
}

impl Output {
    /// Return a new [`OutputBuilder`] with the given [output type](OutputType) and destination. The
    /// blackhole address and deduplicate fields are set to their default values
    /// ([`DEFAULT_BLACKHOLE_ADDRESS_V4`](DEFAULT_BLACKHOLE_ADDRESS_V4) and [`DEFAULT_DEDUPLICATE`](DEFAULT_DEDUPLICATE)
    /// respectively).
    pub fn builder<P>(ty: OutputType, destination: P) -> OutputBuilder
    where
        P: Into<PathBuf>,
    {
        OutputBuilder {
            ty,
            destination: destination.into(),
            blackhole_address: default_blackhole_address(),
            deduplicate: default_deduplicate(),
        }
    }

    /// Returns a reference to the output's [type](OutputType).
    pub fn ty(&self) -> &OutputType {
        &self.ty
    }

    /// Returns a reference to the output's destination.
    pub fn destination(&self) -> &Path {
        self.destination.as_path()
    }

    /// Returns the outputs's blackhole address.
    pub fn blackhole_address(&self) -> IpAddr {
        self.blackhole_address
    }

    /// Returns the outputs's deduplication setting.
    pub fn deduplicate(&self) -> bool {
        self.deduplicate
    }

    pub(crate) fn activate(self) -> Result<ActiveOutput> {
        let mut active_output = ActiveOutput {
            ty: self.ty,
            destination: self.destination,
            blackhole_address: self.blackhole_address,
            deduplicate: self.deduplicate,
            download_dest: tempfile()?,
            seen: HashSet::new(),
        };

        active_output.write_primer()?;
        Ok(active_output)
    }
}

impl ActiveOutput {
    pub fn write_primer(&mut self) -> Result<()> {
        match self.ty {
            OutputType::Hosts { .. } => writeln!(self.download_dest, "# {}", get_generated_at_comment())?,
            OutputType::PdnsLua { .. } => write!(
                self.download_dest,
                "-- {}\n{}",
                get_generated_at_comment(),
                PDNS_LUA_PRIMER
            )?,
        }

        Ok(())
    }

    pub fn write_host(&mut self, host: &str) -> Result<()> {
        if self.deduplicate {
            if self.seen.contains(host) {
                return Ok(());
            }

            self.seen.insert(host.to_string());
        }

        match self.ty {
            OutputType::Hosts { .. } => writeln!(self.download_dest, "{} {}", self.blackhole_address, host)?,
            OutputType::PdnsLua { .. } => {
                // get rid of any comment on the same line as the host
                let host = host.split_once('#').map(|(left, _)| left).unwrap_or(host).trim_end();
                write!(self.download_dest, r#""{}","#, host)?
            }
        }

        Ok(())
    }

    pub fn finalise(&mut self) -> Result<()> {
        match &self.ty {
            OutputType::Hosts { include } => {
                for path in include {
                    let mut include_file = File::open(path)?;
                    writeln!(self.download_dest, "\n# hosts included from {}\n", path.display())?;
                    io::copy(&mut include_file, &mut self.download_dest)?;
                }
            }
            OutputType::PdnsLua {
                output_metric,
                metric_name,
            } => {
                write!(
                    self.download_dest,
                    "}} function preresolve(q) if b:check(q.qname) then "
                )?;

                let record = match self.blackhole_address {
                    IpAddr::V4(_) => "A",
                    IpAddr::V6(_) => "AAAA",
                };

                write!(
                    self.download_dest,
                    "if q.qtype==pdns.{record} then q:addAnswer(pdns.{record},\"{addr}\") ",
                    record = record,
                    addr = self.blackhole_address
                )?;

                if *output_metric {
                    write!(self.download_dest, "m=getMetric(\"{}\") m:inc() ", metric_name)?;
                }

                writeln!(self.download_dest, "return true end end return false end")?;
            }
        }

        // TODO: it'd be nice to get progress callbacks from this
        let mut final_file = File::create(&self.destination)?;
        self.download_dest.seek(SeekFrom::Start(0))?;
        io::copy(&mut self.download_dest, &mut final_file)?;

        Ok(())
    }
}

impl OutputBuilder {
    /// Finalise the builder and return a new [Output].
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The configured destination is an empty path ([SingularityError::EmptyDestination])
    /// - The output's type is a [PDNS Recursor Lua script](OutputType::PdnsLua), its metric is enabled but the metric's
    ///   name is empty.
    pub fn build(self) -> Result<Output> {
        if self.destination.as_os_str().is_empty() {
            return Err(SingularityError::EmptyDestination);
        }

        if let OutputType::PdnsLua {
            output_metric,
            metric_name,
        } = &self.ty
        {
            if *output_metric && metric_name.is_empty() {
                return Err(SingularityError::EmptyMetricName);
            }
        }

        Ok(Output {
            ty: self.ty,
            destination: self.destination,
            blackhole_address: self.blackhole_address,
            deduplicate: self.deduplicate,
        })
    }

    /// Set the builder's blackhole address by attempting to parse a string to an [`IpAddr`]. If you
    /// already have an [`IpAddr`], it is more convenient to use the [`blackhole_ipaddr`](Output::blackhole_ipaddr)
    /// method instead.
    ///
    /// # Errors
    ///
    /// Will return [`SingularityError::InvalidIpAddress`] if parsing the string into an [`IpAddr`] fails.
    ///
    /// [IpAddr]: std::net::IpAddr
    pub fn blackhole_address<S>(mut self, blackhole_address: S) -> Result<Self>
    where
        S: AsRef<str>,
    {
        self.blackhole_address = blackhole_address.as_ref().parse()?;
        Ok(self)
    }

    /// Set the builder's blackhole address. If your address is a string, it is more convenient to use the
    /// [`blackhole_address`](Output::blackhole_address) method instead.
    #[must_use]
    pub fn blackhole_ipaddr<I>(mut self, blackhole_ipaddr: I) -> Self
    where
        I: Into<IpAddr>,
    {
        self.blackhole_address = blackhole_ipaddr.into();
        self
    }

    /// Set the builder's deduplication setting.
    #[must_use]
    pub fn deduplicate(mut self, deduplicate: bool) -> Self {
        self.deduplicate = deduplicate;
        self
    }
}

fn get_generated_at_comment() -> String {
    format!(
        "Generated at {} with {} v{}",
        Local::now(),
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
}

fn default_blackhole_address() -> IpAddr {
    DEFAULT_BLACKHOLE_ADDRESS_V4
}

#[cfg(feature = "serde")]
fn default_output_metric() -> bool {
    DEFAULT_OUTPUT_METRIC
}

#[cfg(feature = "serde")]
fn default_metric_name() -> String {
    String::from(DEFAULT_METRIC_NAME)
}

fn default_deduplicate() -> bool {
    DEFAULT_DEDUPLICATE
}

#[cfg(test)]
mod tests {
    use super::{Output, OutputType};
    use crate::{SingularityError, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_METRIC_NAME};
    use std::io::{Read, Write};
    use tempfile::NamedTempFile;

    #[test]
    fn default_fields() {
        let output = Output::builder(OutputType::Hosts { include: vec![] }, "path").build();
        assert!(output.is_ok());
    }

    #[test]
    fn all_valid_fields() {
        let output = Output::builder(
            OutputType::PdnsLua {
                output_metric: true,
                metric_name: String::from("test"),
            },
            "path",
        )
        .blackhole_address("1.2.3.4")
        .expect("failed to set blackhole address")
        .deduplicate(true)
        .build();
        assert!(output.is_ok());
    }

    #[test]
    fn empty_destination() {
        let output = Output::builder(OutputType::Hosts { include: vec![] }, "").build();
        assert!(matches!(output, Err(SingularityError::EmptyDestination)));
    }

    #[test]
    fn empty_lua_metric_name() {
        let output = Output::builder(
            OutputType::PdnsLua {
                output_metric: true,
                metric_name: String::new(),
            },
            "path",
        )
        .build();
        assert!(matches!(output, Err(SingularityError::EmptyMetricName)));
    }

    #[test]
    fn invalid_blackhole_address() {
        let builder =
            Output::builder(OutputType::Hosts { include: vec![] }, "path").blackhole_address("invalid ip address");
        assert!(matches!(builder, Err(SingularityError::InvalidIpAddress(_))));
    }

    #[test]
    fn activation() {
        let output = Output::builder(OutputType::Hosts { include: vec![] }, "path")
            .build()
            .expect("failed to build output");

        let activate = output.activate();
        assert!(activate.is_ok());
    }

    #[test]
    fn write_host() {
        let mut output = Output::builder(OutputType::Hosts { include: vec![] }, "path")
            .build()
            .expect("failed to build output")
            .activate()
            .expect("failed to activate output");

        let write = output.write_host("host");
        assert!(write.is_ok());
    }

    #[test]
    fn finalise_hosts() {
        let mut dest = NamedTempFile::new().expect("failed to create dest tempfile");
        let path = dest.path();

        println!("Using dest: {}", path.display());
        let mut output = Output::builder(OutputType::Hosts { include: vec![] }, path)
            .build()
            .expect("failed to build output")
            .activate()
            .expect("failed to activate output");

        output.write_host("example.com").expect("failed to write host");
        output.write_host("google.com").expect("failed to write host");
        output.finalise().expect("failed to finalise output");

        let mut buf = String::new();
        dest.read_to_string(&mut buf).expect("failed to read output");

        // the generated-at comment cannot be tested for
        let out = buf.split_once('\n').expect("failed to split output").1;

        assert_eq!(
            out,
            format!("{bh} example.com\n{bh} google.com\n", bh = DEFAULT_BLACKHOLE_ADDRESS_V4)
        );
    }

    #[test]
    fn finalise_pdns_lua() {
        let mut dest = NamedTempFile::new().expect("failed to create dest tempfile");
        let path = dest.path();

        println!("Using dest: {}", path.display());
        let mut output = Output::builder(
            OutputType::PdnsLua {
                output_metric: true,
                metric_name: String::from(DEFAULT_METRIC_NAME),
            },
            path,
        )
        .build()
        .expect("failed to build output")
        .activate()
        .expect("failed to activate output");

        output.write_host("example.com").expect("failed to write host");
        output.write_host("google.com").expect("failed to write host");
        output.finalise().expect("failed to finalise output");

        let mut buf = String::new();
        dest.read_to_string(&mut buf).expect("failed to read output");

        // the generated-at comment cannot be tested for
        let script = buf.split_once('\n').expect("failed to split output").1;

        assert_eq!(
            script,
            format!(
                "b=newDS() b:add{{\"example.com\",\"google.com\",}} function preresolve(q) if b:check(q.qname) then \
                 if q.qtype==pdns.A then q:addAnswer(pdns.A,\"{}\") m=getMetric(\"{}\") m:inc() return true end end \
                 return false end\n",
                DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_METRIC_NAME
            )
        );
    }

    #[test]
    fn finalise_pdns_lua_no_metric() {
        let mut dest = NamedTempFile::new().expect("failed to create dest tempfile");
        let path = dest.path();

        println!("Using dest: {}", path.display());
        let mut output = Output::builder(
            OutputType::PdnsLua {
                output_metric: false,
                metric_name: String::from(DEFAULT_METRIC_NAME),
            },
            path,
        )
        .build()
        .expect("failed to build output")
        .activate()
        .expect("failed to activate output");

        output.write_host("example.com").expect("failed to write host");
        output.write_host("google.com").expect("failed to write host");
        output.finalise().expect("failed to finalise output");

        let mut buf = String::new();
        dest.read_to_string(&mut buf).expect("failed to read output");

        // the generated-at comment cannot be tested for
        let script = buf.split_once('\n').expect("failed to split output").1;

        assert_eq!(
            script,
            format!(
                "b=newDS() b:add{{\"example.com\",\"google.com\",}} function preresolve(q) if b:check(q.qname) then \
                 if q.qtype==pdns.A then q:addAnswer(pdns.A,\"{}\") return true end end return false end\n",
                DEFAULT_BLACKHOLE_ADDRESS_V4
            )
        );
    }

    #[test]
    fn deduplication() {
        let mut dest = NamedTempFile::new().expect("failed to create dest tempfile");
        let path = dest.path();

        println!("Using dest: {}", path.display());
        let mut output = Output::builder(OutputType::Hosts { include: vec![] }, path)
            .deduplicate(true)
            .build()
            .expect("failed to build output")
            .activate()
            .expect("failed to activate output");

        output.write_host("example.com").expect("failed to write host");
        output.write_host("example.com").expect("failed to write host");
        output.finalise().expect("failed to finalise output");

        let mut buf = String::new();
        dest.read_to_string(&mut buf).expect("failed to read output");

        // the generated-at comment cannot be tested for
        let out = buf.split_once('\n').expect("failed to split output").1;

        assert_eq!(out, format!("{} example.com\n", DEFAULT_BLACKHOLE_ADDRESS_V4));
    }

    #[test]
    fn include_hosts() {
        const INCLUDE_HOSTS: &str = "0.0.0.0 google.com";

        let mut include = NamedTempFile::new().expect("failed to create include tempfile");
        include
            .write_all(INCLUDE_HOSTS.as_bytes())
            .expect("failed to write include hosts");

        let mut dest = NamedTempFile::new().expect("failed to create dest tempfile");
        let path = dest.path();

        println!("Using dest: {}", path.display());
        let mut output = Output::builder(
            OutputType::Hosts {
                include: vec![include.path().to_path_buf()],
            },
            path,
        )
        .build()
        .expect("failed to build output")
        .activate()
        .expect("failed to activate output");

        output.write_host("example.com").expect("failed to write host");
        output.finalise().expect("failed to finalise output");

        let mut buf = String::new();
        dest.read_to_string(&mut buf).expect("failed to read output");

        // the generated-at comment cannot be tested for
        let out = buf.split_once('\n').expect("failed to split output").1;

        assert_eq!(
            out,
            format!(
                "{bh} example.com\n\n# hosts included from {inc}\n\n{bh} google.com",
                bh = DEFAULT_BLACKHOLE_ADDRESS_V4,
                inc = include.path().display()
            )
        );
    }
}
