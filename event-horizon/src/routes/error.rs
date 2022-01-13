use crate::{template, ErrorProvider};
use actix_web::{
    http::{header, StatusCode},
    web, HttpResponse, Responder,
};
use log::*;
use std::sync::RwLock;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/error")
            .route("/", web::get().to(redirect))
            .route("/{id}", web::get().to(error)),
    );
}

async fn redirect() -> impl Responder {
    HttpResponse::build(StatusCode::MOVED_PERMANENTLY)
        .append_header((header::LOCATION, "/"))
        .finish()
}

async fn error(error_id: web::Path<String>, error_provider: web::Data<RwLock<ErrorProvider>>) -> impl Responder {
    let provider = error_provider.read().expect("error provider rwlock is poisoned");

    match provider.get_ref(&error_id) {
        Some(msg) => template::error(msg).ok(),
        None => {
            warn!("No such error ID: {}", error_id);
            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/"))
                .finish()
        }
    }
}
