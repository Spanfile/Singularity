mod danger_zone;
mod import_singularity_config;

use crate::template::{
    self,
    settings::{EventHorizonSubPage, SettingsPage},
};
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/event_horizon")
            .route("", web::get().to(event_horizon))
            .configure(import_singularity_config::config)
            .configure(danger_zone::config),
    );
}

async fn event_horizon() -> impl Responder {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::Main)).ok()
}
