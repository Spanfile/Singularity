use crate::config::{OutputConfig, OutputConfigType};
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

const PDNS_LUA_PRIMER: &str = "b=newDS() b:add{";

enum OutputType {
    Hosts(Vec<PathBuf>),
    PdnsLua { output_metric: bool, metric_name: String },
}

pub(crate) struct Output {
    ty: OutputType,
    destination: File,
    final_path: PathBuf,
    blackhole_address: IpAddr,
    deduplicate: bool,
    seen: HashSet<String>,
}

impl Output {
    pub fn from_config(cfg: &OutputConfig) -> anyhow::Result<Self> {
        match &cfg.ty {
            OutputConfigType::Hosts { include } => Ok(Self {
                ty: OutputType::Hosts(include.to_owned()),
                destination: tempfile()?,
                final_path: cfg.destination.to_path_buf(),
                blackhole_address: cfg.blackhole_address,
                deduplicate: cfg.deduplicate,
                seen: HashSet::new(),
            }),
            OutputConfigType::PdnsLua {
                output_metric,
                metric_name,
            } => Ok(Self {
                ty: OutputType::PdnsLua {
                    output_metric: *output_metric,
                    metric_name: metric_name.to_owned(),
                },
                destination: tempfile()?,
                final_path: cfg.destination.to_path_buf(),
                blackhole_address: cfg.blackhole_address,
                deduplicate: cfg.deduplicate,
                seen: HashSet::new(),
            }),
        }
    }

    pub fn write_primer(&mut self) -> anyhow::Result<()> {
        match self.ty {
            OutputType::Hosts(_) => writeln!(self.destination, "# {}", get_generated_at_comment())?,
            OutputType::PdnsLua { .. } => write!(
                self.destination,
                "-- {}\n{}",
                get_generated_at_comment(),
                PDNS_LUA_PRIMER
            )?,
        }

        Ok(())
    }

    pub fn write_host(&mut self, host: &str) -> anyhow::Result<()> {
        if self.deduplicate {
            if self.seen.contains(host) {
                return Ok(());
            }
            self.seen.insert(host.to_string());
        }
        match self.ty {
            OutputType::Hosts(_) => writeln!(self.destination, "{} {}", self.blackhole_address, host)?,
            OutputType::PdnsLua { .. } => {
                let host = host.split_once('#').map(|(left, _)| left).unwrap_or(host).trim_end();
                write!(self.destination, r#""{}","#, host)?
            }
        }

        Ok(())
    }

    pub fn finalise(mut self) -> anyhow::Result<()> {
        match self.ty {
            OutputType::Hosts(include) => {
                for path in &include {
                    let mut include_file = File::open(path)?;
                    writeln!(self.destination, "\n# hosts included from {}\n", path.display())?;
                    io::copy(&mut include_file, &mut self.destination)?;
                }
            }
            OutputType::PdnsLua {
                output_metric,
                metric_name,
            } => {
                write!(self.destination, "}} function preresolve(q) if b:check(q.qname) then ")?;

                let record = match self.blackhole_address {
                    IpAddr::V4(_) => "A",
                    IpAddr::V6(_) => "AAAA",
                };

                write!(
                    self.destination,
                    "if q.qtype==pdns.{record} then q:addAnswer(pdns.{record},\"{addr}\") ",
                    record = record,
                    addr = self.blackhole_address
                )?;

                if output_metric {
                    write!(self.destination, "m=getMetric(\"{}\") m:inc() ", metric_name)?;
                }

                writeln!(self.destination, "return true end end return false end")?;
            }
        }

        let mut final_file = File::create(&self.final_path)?;
        self.destination.seek(SeekFrom::Start(0))?;
        io::copy(&mut self.destination, &mut final_file)?;

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
