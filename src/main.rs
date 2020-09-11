mod logging;

use std::path::PathBuf;

use log::*;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use url::Url;

const APP_NAME: &str = "pdns-singularity";

#[derive(Debug, StructOpt)]
#[structopt(
    name = APP_NAME,
    about = "Gathers blacklisted DNS domains into a PDNS Recursor hosts-file."
)]
struct Opt {
    /// Enable verbose logging
    #[structopt(short, long)]
    verbose: bool,
    /// Custom path to the app's configuration file.
    #[structopt(short, long)]
    config: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Config {
    adlists: Vec<AdlistConf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AdlistConf {
    source: Url,
    format: AdlistFormat,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AdlistFormat {
    Hosts,
    Domains,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    logging::setup_logging(if opt.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    })?;

    let cfg: Config = match &opt.config {
        Some(path) => confy::load_path(path)?,
        None => confy::load(APP_NAME)?,
    };

    debug!("{:?}", opt);
    debug!("{:?}", cfg);

    Ok(())
}
