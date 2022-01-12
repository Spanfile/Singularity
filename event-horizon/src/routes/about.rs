use crate::template;
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/about").route(web::get().to(about)));
}

async fn about() -> impl Responder {
    template::about().current_path("/about").ok()
}
