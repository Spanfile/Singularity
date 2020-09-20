use crate::{
    config::{OutputConfig, OutputConfigType},
    split_once,
};
use chrono::Local;
use io::SeekFrom;
use std::{
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
    PdnsLua,
}

pub(crate) struct Output {
    ty: OutputType,
    destination: File,
    final_path: PathBuf,
    blackhole_address: IpAddr,
}

impl Output {
    pub fn from_config(cfg: &OutputConfig) -> anyhow::Result<Self> {
        match &cfg.ty {
            OutputConfigType::Hosts { include } => Ok(Self {
                ty: OutputType::Hosts(include.to_owned()),
                destination: tempfile()?,
                final_path: cfg.destination.to_path_buf(),
                blackhole_address: cfg.blackhole_address,
            }),
            OutputConfigType::PdnsLua => Ok(Self {
                ty: OutputType::PdnsLua,
                destination: tempfile()?,
                final_path: cfg.destination.to_path_buf(),
                blackhole_address: cfg.blackhole_address,
            }),
        }
    }

    pub fn write_primer(&mut self) -> anyhow::Result<()> {
        match self.ty {
            OutputType::Hosts(_) => writeln!(&mut self.destination, "# {}", get_generated_at_comment())?,
            OutputType::PdnsLua => write!(
                &mut self.destination,
                "-- {}\n{}",
                get_generated_at_comment(),
                PDNS_LUA_PRIMER
            )?,
        }

        Ok(())
    }

    pub fn write_host(&mut self, host: &str) -> anyhow::Result<()> {
        match self.ty {
            OutputType::Hosts(..) => writeln!(&mut self.destination, "{} {}", self.blackhole_address, host)?,
            OutputType::PdnsLua => {
                let host = split_once(&host, "#").map(|(left, _)| left).unwrap_or(host).trim_end();
                write!(&mut self.destination, r#""{}","#, host)?
            }
        }

        Ok(())
    }

    pub fn finalise(&mut self) -> anyhow::Result<()> {
        match &self.ty {
            OutputType::Hosts(include) => {
                for path in include {
                    let mut include_file = File::open(path)?;
                    writeln!(&mut self.destination, "\n# hosts included from {}\n", path.display())?;
                    io::copy(&mut include_file, &mut self.destination)?;
                }
            }
            OutputType::PdnsLua => {
                write!(
                    &mut self.destination,
                    "}} function preresolve(q) if b:check(q.qname) then "
                )?;

                let record = match self.blackhole_address {
                    IpAddr::V4(_) => "A",
                    IpAddr::V6(_) => "AAAA",
                };

                write!(
                    &mut self.destination,
                    "if q.qtype==pdns.{record} then q:addAnswer(pdns.{record},\"{addr}\")",
                    record = record,
                    addr = self.blackhole_address
                )?;

                writeln!(&mut self.destination, " return true end end return false end")?;
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
