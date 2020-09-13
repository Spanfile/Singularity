use crate::config::OutputConfig;
use chrono::Utc;
use std::{fs::File, io, io::Write, net::IpAddr, path::PathBuf};

const PDNS_LUA_PRIMER: &str = "b=newDS();b:add{";

enum OutputType {
    Hosts(Vec<PathBuf>),
    PdnsLua,
}

pub(crate) struct Output {
    ty: OutputType,
    destination: File,
    blackhole_address: IpAddr,
}

impl Output {
    pub fn from_config(output_config: &OutputConfig) -> anyhow::Result<Self> {
        match output_config {
            OutputConfig::Hosts {
                destination,
                blackhole_address,
                include,
            } => Ok(Self {
                ty: OutputType::Hosts(include.to_owned()),
                destination: File::create(destination)?,
                blackhole_address: *blackhole_address,
            }),
            OutputConfig::PdnsLua {
                destination,
                blackhole_address,
            } => Ok(Self {
                ty: OutputType::PdnsLua,
                destination: File::create(destination)?,
                blackhole_address: *blackhole_address,
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
                let host = host.split_once('#').map(|(left, _)| left).unwrap_or(host);
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
                    writeln!(&mut self.destination, "\nhosts included from {}\n", path.display())?;
                    io::copy(&mut include_file, &mut self.destination)?;
                }
            }
            OutputType::PdnsLua => {
                write!(
                    &mut self.destination,
                    "}};function preresolve(q) if b:check(q.qname) then "
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

                writeln!(&mut self.destination, "return true end end return false end")?;
            }
        }

        Ok(())
    }
}

fn get_generated_at_comment() -> String {
    format!("Generated at {} by singularity", Utc::now().to_rfc3339())
}
