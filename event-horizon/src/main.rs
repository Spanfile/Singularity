#[macro_use]
extern crate diesel;

mod config;
mod database;
mod error;
mod logging;
mod rec_control;
mod routes;
mod singularity;
mod template;
mod util;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use crate::{
    config::{EnvConfig, EvhConfig, Listen},
    database::DbPool,
    error::{EvhError, EvhResult},
    singularity::{ConfigImporter, ConfigManager},
};
use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};
use database::RedisPool;
use diesel::{
    r2d2::{self, ConnectionManager},
    SqliteConnection,
};
use log::*;
use rec_control::RecControl;
use std::time::Duration;

#[actix_web::main]
async fn main() -> EvhResult<()> {
    if cfg!(debug_assertions) {
        dotenv::dotenv().unwrap();
    }

    let env_config = EnvConfig::load()?;
    logging::setup_logging(&env_config)?;

    let evh_config = EvhConfig::load(&env_config.config)?;

    debug!("Env: {:#?}", env_config);
    debug!("EVH: {:#?}", evh_config);

    let db_pool = create_db_pool(&evh_config)?;
    let redis_pool = create_redis_pool(&evh_config)?;
    let rec_control = create_rec_control(&evh_config).await?;

    let mut conn = db_pool.get()?;
    let cfg_manager = ConfigManager::load(&mut conn)?;
    let config_importer = web::Data::new(ConfigImporter::new(&evh_config));

    let env_config = web::Data::new(env_config);
    let evh_config = web::Data::new(evh_config);
    let db_pool = web::Data::new(db_pool);
    let redis_pool = web::Data::new(redis_pool);
    let rec_control = web::Data::new(rec_control);
    let cfg_manager = web::Data::new(cfg_manager);

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
            .app_data(db_pool.clone())
            .app_data(redis_pool.clone())
            .app_data(rec_control.clone())
            .app_data(cfg_manager.clone())
            .app_data(config_importer.clone())
            .service(Files::new("/static", "static/"))
            .configure(routes::index::config)
            .configure(routes::about::config)
            .configure(routes::settings::config)
            .configure(routes::stats::config)
    })
    .bind(listener)?
    .run()
    .await?;

    Ok(())
}

fn create_db_pool(evh_config: &EvhConfig) -> EvhResult<DbPool> {
    // TODO: run migrations

    debug!("Establishing SQLite connection to {}", evh_config.database_url);

    let manager = ConnectionManager::<SqliteConnection>::new(&evh_config.database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .map_err(EvhError::DatabasePoolInitialisationFailed)?;

    debug!("{:#?}", pool);
    Ok(DbPool::new(pool))
}

fn create_redis_pool(evh_config: &EvhConfig) -> EvhResult<RedisPool> {
    debug!("Establishing Redis connection to {}", evh_config.redis.redis_url);

    // the redis client is kinda silly in that it doesn't allow &Url, only Url, so just fuckin' clone the thing
    let client = redis::Client::open(evh_config.redis.redis_url.clone())?;
    debug!("{:#?}", client);

    // test the connection
    let mut conn = client.get_connection_with_timeout(Duration::from_millis(evh_config.redis.connection_timeout))?;
    let pong: String = redis::cmd("PING").query(&mut conn)?;
    debug!("Testing Redis connection: {}", pong);

    let pool = r2d2::Pool::builder()
        .build(client)
        .map_err(EvhError::RedisPoolInitialisationFailed)?;
    Ok(RedisPool::new(pool))
}

async fn create_rec_control(evh_config: &EvhConfig) -> EvhResult<RecControl> {
    debug!(
        "Establishing connection to Recursor control socket {}",
        evh_config.recursor.control_socket.display()
    );

    RecControl::new(&evh_config.recursor.control_socket).await
}
