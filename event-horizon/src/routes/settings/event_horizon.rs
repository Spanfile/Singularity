mod danger_zone;
mod import_singularity_config;

use crate::{
    database::DbPool,
    error::EvhError,
    singularity::SingularityConfig,
    template::{
        self,
        settings::{EventHorizonSubPage, SettingsPage},
        Alert,
    },
};
use actix_web::{web, Responder};
use log::*;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/event_horizon")
            .route("", web::get().to(event_horizon))
            .configure(import_singularity_config::config)
            .configure(danger_zone::config),
    );
}

async fn event_horizon(db_pool: web::Data<DbPool>) -> impl Responder {
    match web::block(move || {
        let mut conn = db_pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
        SingularityConfig::load_all(&mut conn)
    })
    .await
    .unwrap()
    {
        Ok(cfgs) => template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::Main(Some(&cfgs)))),
        Err(e) => {
            error!("Failed to load all Singularity configurations: {}", e);

            template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::Main(None))).alert(Alert::Error(
                format!(
                    "Failed to load all Singularity configurations due to an internal server error: {}",
                    e
                ),
            ))
        }
    }
}
