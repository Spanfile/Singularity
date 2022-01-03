use crate::template;
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(about);
}

#[actix_web::get("/about")]
async fn about() -> impl Responder {
    template::about::about()
}
