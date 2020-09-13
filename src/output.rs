use crate::config::OutputConfig;
use std::{fs::File, io, io::Write, net::IpAddr, path::PathBuf};

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

    pub fn write(&mut self, domain: &str) -> anyhow::Result<()> {
        match self.ty {
            OutputType::Hosts(..) => writeln!(&mut self.destination, "{} {}", self.blackhole_address, domain)?,
            OutputType::PdnsLua => unimplemented!(),
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
            OutputType::PdnsLua => {}
        }

        Ok(())
    }
}
