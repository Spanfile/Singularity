mod import_singularity_config;

use crate::{
    config::EvhConfig,
    template::{
        self,
        settings::{EventHorizonSubPage, SettingsPage},
    },
};
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/event_horizon")
            .route("", web::get().to(event_horizon))
            .configure(import_singularity_config::config),
    );
}

async fn event_horizon(evh_config: web::Data<EvhConfig>) -> impl Responder {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::Main {
        evh_config: &evh_config,
    }))
    .ok()
}
