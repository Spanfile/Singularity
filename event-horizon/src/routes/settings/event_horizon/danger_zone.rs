use crate::{
    config::{EnvConfig, EvhConfig},
    template::{
        self,
        settings::{EventHorizonSubPage, SettingsPage},
    },
};
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/danger_zone").route("", web::get().to(danger_zone)));
}

async fn danger_zone(evh_config: web::Data<EvhConfig>, env_config: web::Data<EnvConfig>) -> impl Responder {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::DangerZone {
        evh_config: &evh_config,
        env_config: &env_config,
    }))
    .ok()
}
