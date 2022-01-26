mod danger_zone;
mod import_singularity_config;
mod modify_singularity_config;

use crate::{
    database::DbPool,
    error::EvhError,
    singularity::{ConfigManager, SingularityConfig},
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
            .route("", web::get().to(event_horizon_page))
            .configure(import_singularity_config::config)
            .configure(modify_singularity_config::config)
            .configure(danger_zone::config),
    );
}

async fn event_horizon_page(cfg_mg: web::Data<ConfigManager>, db_pool: web::Data<DbPool>) -> impl Responder {
    let active_cfg = cfg_mg.get_active_config().id();

    match web::block(move || {
        let mut conn = db_pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
        SingularityConfig::load_all(&mut conn)
    })
    .await
    .unwrap()
    {
        Ok(cfgs) => template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::Main {
            cfgs: Some(&cfgs),
            active_cfg,
        })),
        Err(e) => {
            error!("Failed to load all Singularity configurations: {}", e);

            template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::Main {
                cfgs: None,
                active_cfg,
            }))
            .alert(Alert::Error(format!(
                "Failed to load all Singularity configurations due to an internal server error: {}",
                e
            )))
        }
    }
}
