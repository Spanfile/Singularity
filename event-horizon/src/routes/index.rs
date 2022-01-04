use crate::template;
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/").service(index));
}

#[actix_web::get("")]
async fn index() -> impl Responder {
    template::index().current_path("/").build()
}
