mod config;
mod logging;
mod routes;
mod singularity;
mod template;
mod util;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use crate::{
    config::{Config, Listen},
    singularity::SingularityConfig,
};
use ::singularity::{Adlist, AdlistFormat};
use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};
use log::*;
use std::sync::RwLock;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    if cfg!(debug_assertions) {
        dotenv::dotenv().unwrap();
    }

    let config = Config::load()?;
    logging::setup_logging(&config)?;

    debug!("{:#?}", config);

    let listener = match config.listen {
        Listen::Http { bind } => bind,
        Listen::Https {
            bind: _,
            tls_certificate: _,
            tls_certificate_key: _,
        } => unimplemented!(),
        Listen::Unix { bind: _ } => unimplemented!(),
    };

    let config = web::Data::new(config);
    let mut singularity_config = SingularityConfig::default();

    // add some dummy data
    singularity_config.add_adlist(Adlist::new(
        "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
        AdlistFormat::Hosts,
    )?);
    singularity_config.add_adlist(Adlist::new(
        "https://raw.githubusercontent.com/VeleSila/yhosts/master/hosts",
        AdlistFormat::Hosts,
    )?);
    singularity_config.add_adlist(Adlist::new(
        "https://github.com/notracking/hosts-blocklists/raw/master/dnsmasq/dnsmasq.blacklist.txt",
        AdlistFormat::Dnsmasq,
    )?);

    let singularity_config = web::Data::new(RwLock::new(singularity_config));

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(config.clone())
            .app_data(singularity_config.clone())
            .service(Files::new("/static", "static/"))
            .configure(routes::index::config)
            .configure(routes::about::config)
            .configure(routes::settings::config)
    })
    .bind(listener)?
    .run()
    .await?;

    Ok(())
}
