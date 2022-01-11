#[macro_use]
extern crate diesel;

mod config;
mod database;
mod error;
mod logging;
mod routes;
mod singularity;
mod template;
mod util;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use crate::{
    config::{EnvConfig, EvhConfig, Listen},
    singularity::{ConfigImporter, SingularityConfig},
};
use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};
use database::DbPool;
use diesel::{
    r2d2::{self, ConnectionManager},
    SqliteConnection,
};
use log::*;
use std::sync::RwLock;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    if cfg!(debug_assertions) {
        dotenv::dotenv().unwrap();
    }

    let env_config = EnvConfig::load()?;
    let evh_config = EvhConfig::load()?;

    logging::setup_logging(&env_config)?;

    debug!("Env: {:#?}", env_config);
    debug!("EVH: {:#?}", evh_config);

    let pool = create_db_pool(&evh_config)?;
    let mut conn = pool.get()?;

    // attempt to load the config with ID 1, or if it fails because it doesn't exist, attempt to create a new config
    let singularity_config = SingularityConfig::load(1, &mut conn).or_else(|e| {
        if let Some(diesel::result::Error::NotFound) = e.downcast_ref() {
            warn!("No existing Singularity config found, falling back to creating a new one");
            SingularityConfig::new(&mut conn)
        } else {
            Err(e)
        }
    })?;

    // add some dummy data
    // singularity_config.add_adlist(Adlist::new(
    //     "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts",
    //     AdlistFormat::Hosts,
    // )?);

    // singularity_config.add_adlist(Adlist::new(
    //     "https://raw.githubusercontent.com/VeleSila/yhosts/master/hosts",
    //     AdlistFormat::Hosts,
    // )?);

    // singularity_config.add_adlist(Adlist::new(
    //     "https://github.com/notracking/hosts-blocklists/raw/master/dnsmasq/dnsmasq.blacklist.txt",
    //     AdlistFormat::Dnsmasq,
    // )?);

    // singularity_config.add_output(
    //     Output::builder(
    //         OutputType::PdnsLua {
    //             output_metric: false,
    //             metric_name: String::from("metric"),
    //         },
    //         "test/path",
    //     )
    //     .build()
    //     .unwrap(),
    // );

    // singularity_config.add_output(
    //     Output::builder(
    //         OutputType::Hosts {
    //             include: vec!["hosts1".into(), "hosts2".into(), "hosts3".into()],
    //         },
    //         "test/path",
    //     )
    //     .build()
    //     .unwrap(),
    // );

    let env_config = web::Data::new(env_config);
    let evh_config = web::Data::new(evh_config);
    let pool = web::Data::new(pool);
    let singularity_config = web::Data::new(singularity_config);
    let config_importer = web::Data::new(RwLock::new(ConfigImporter::new()));

    let listener = match env_config.listen {
        Listen::Http { bind } => bind,
        Listen::Https {
            bind: _,
            tls_certificate: _,
            tls_certificate_key: _,
        } => unimplemented!(),
        Listen::Unix { bind: _ } => unimplemented!(),
    };

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(env_config.clone())
            .app_data(evh_config.clone())
            .app_data(pool.clone())
            .app_data(singularity_config.clone())
            .app_data(config_importer.clone())
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

fn create_db_pool(evh_config: &EvhConfig) -> anyhow::Result<DbPool> {
    let manager = ConnectionManager::<SqliteConnection>::new(&evh_config.database_url);
    let pool = r2d2::Pool::builder().build(manager)?;
    Ok(pool)
}
