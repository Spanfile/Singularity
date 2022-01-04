use crate::template::{self, Alert};
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/settings").service(settings));
}

#[actix_web::get("")]
async fn settings() -> impl Responder {
    template::settings().current_path("/settings").build()
}
