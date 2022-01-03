mod config;
mod logging;
mod routes;
mod template;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use crate::config::{Config, Listen};
use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};
use log::*;
use std::sync::Arc;

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

    let config = Arc::new(config);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(Arc::clone(&config)))
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
