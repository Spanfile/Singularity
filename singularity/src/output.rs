use crate::Result;
use chrono::Local;
use io::SeekFrom;
use std::{
    collections::HashSet,
    fs::File,
    io,
    io::{Seek, Write},
    net::IpAddr,
    path::PathBuf,
};
use tempfile::tempfile;

pub const DEFAULT_BLACKHOLE_ADDRESS_V4: &str = "0.0.0.0";
pub const DEFAULT_BLACKHOLE_ADDRESS_V6: &str = "::";
const PDNS_LUA_PRIMER: &str = "b=newDS() b:add{";

#[derive(Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Output {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: OutputType,
    pub destination: PathBuf,
    #[cfg_attr(feature = "serde", serde(default = "default_blackhole_address"))]
    pub blackhole_address: IpAddr,
    #[cfg_attr(feature = "serde", serde(default = "default_deduplicate"))]
    pub deduplicate: bool,
}

#[derive(Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize), serde(tag = "type"))]
pub enum OutputType {
    #[cfg_attr(feature = "serde", serde(rename = "hosts"))]
    Hosts {
        #[cfg_attr(feature = "serde", serde(default))]
        include: Vec<PathBuf>,
    },
    #[cfg_attr(feature = "serde", serde(rename = "pdns-lua"))]
    PdnsLua {
        #[cfg_attr(feature = "serde", serde(default = "default_output_metric"))]
        output_metric: bool,
        #[cfg_attr(feature = "serde", serde(default = "default_metric_name"))]
        metric_name: String,
    },
}

#[derive(Debug)]
pub(crate) struct ActiveOutput {
    ty: OutputType,
    destination: PathBuf,
    blackhole_address: IpAddr,
    deduplicate: bool,
    download_dest: File,
    seen: HashSet<String>,
}

impl Output {
    pub fn new<P>(ty: OutputType, destination: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            ty,
            destination: destination.into(),
            blackhole_address: default_blackhole_address(),
            deduplicate: default_deduplicate(),
        }
    }

    #[must_use]
    pub fn blackhole_address<I>(mut self, blackhole_address: I) -> Self
    where
        I: Into<IpAddr>,
    {
        self.blackhole_address = blackhole_address.into();
        self
    }

    #[must_use]
    pub fn deduplicate(mut self, deduplicate: bool) -> Self {
        self.deduplicate = deduplicate;
        self
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
            OutputType::Hosts { .. } => writeln!(&mut self.download_dest, "# {}", get_generated_at_comment())?,
            OutputType::PdnsLua { .. } => write!(
                &mut self.download_dest,
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
            OutputType::Hosts { .. } => writeln!(&mut self.download_dest, "{} {}", self.blackhole_address, host)?,
            OutputType::PdnsLua { .. } => {
                // get rid of any comment on the same line as the host
                let host = host.split_once('#').map(|(left, _)| left).unwrap_or(host).trim_end();
                write!(&mut self.download_dest, r#""{}","#, host)?
            }
        }

        Ok(())
    }

    pub fn finalise(mut self) -> Result<()> {
        match self.ty {
            OutputType::Hosts { include } => {
                for path in &include {
                    let mut include_file = File::open(path)?;
                    writeln!(&mut self.download_dest, "\n# hosts included from {}\n", path.display())?;
                    io::copy(&mut include_file, &mut self.download_dest)?;
                }
            }
            OutputType::PdnsLua {
                output_metric,
                metric_name,
            } => {
                write!(
                    &mut self.download_dest,
                    "}} function preresolve(q) if b:check(q.qname) then "
                )?;

                let record = match self.blackhole_address {
                    IpAddr::V4(_) => "A",
                    IpAddr::V6(_) => "AAAA",
                };

                write!(
                    &mut self.download_dest,
                    "if q.qtype==pdns.{record} then q:addAnswer(pdns.{record},\"{addr}\") ",
                    record = record,
                    addr = self.blackhole_address
                )?;

                if output_metric {
                    write!(&mut self.download_dest, "m=getMetric(\"{}\") m:inc() ", metric_name)?;
                }

                writeln!(&mut self.download_dest, "return true end end return false end")?;
            }
        }

        let mut final_file = File::create(&self.destination)?;
        self.download_dest.seek(SeekFrom::Start(0))?;
        io::copy(&mut self.download_dest, &mut final_file)?;

        Ok(())
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
        .parse()
        .expect("failed to parse default blackhole address")
}

#[cfg(feature = "serde")]
fn default_output_metric() -> bool {
    true
}

#[cfg(feature = "serde")]
fn default_metric_name() -> String {
    String::from("blocked-queries")
}

fn default_deduplicate() -> bool {
    false
}
