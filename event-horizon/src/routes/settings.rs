mod singularity;

use crate::template::{self, settings::SettingsPage};
use actix_web::{http::header, web, HttpResponse, Responder};

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
async fn event_horizon() -> impl Responder {
    template::settings(SettingsPage::EventHorizon).ok()
}

#[actix_web::get("/recursor")]
async fn recursor() -> impl Responder {
    template::settings(SettingsPage::Recursor).ok()
}
