mod singularity;

use crate::{
    singularity::SingularityConfig,
    template::{self, settings::SettingsPage},
};
use actix_web::{http::header, web, HttpResponse, Responder};
use std::sync::RwLock;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/settings")
            .configure(singularity::config)
            .service(settings)
            .service(event_horizon)
            .service(recursor),
    );
}

#[actix_web::get("")]
async fn settings() -> impl Responder {
    HttpResponse::MovedPermanently()
        .customize()
        .insert_header((header::LOCATION, "/settings/eventhorizon"))
}

#[actix_web::get("/eventhorizon")]
async fn event_horizon(singularity_config: web::Data<RwLock<SingularityConfig>>) -> impl Responder {
    let cfg = singularity_config
        .read()
        .expect("failed to lock read singularity config");

    template::settings(SettingsPage::EventHorizon, &cfg).ok()
}

#[actix_web::get("/recursor")]
async fn recursor(singularity_config: web::Data<RwLock<SingularityConfig>>) -> impl Responder {
    let cfg = singularity_config
        .read()
        .expect("failed to lock read singularity config");

    template::settings(SettingsPage::Recursor, &cfg).ok()
}
