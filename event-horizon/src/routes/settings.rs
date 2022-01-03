use crate::template;
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(settings);
}

#[actix_web::get("/settings")]
async fn settings() -> impl Responder {
    template::settings::settings()
}
