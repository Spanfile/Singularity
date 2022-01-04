use crate::template::{self, Alert};
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/about").service(about));
}

#[actix_web::get("")]
async fn about() -> impl Responder {
    template::about().current_path("/about").build()
}
