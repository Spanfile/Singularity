use crate::template;
use actix_web::{
    http::{header, StatusCode},
    web, HttpResponse, Responder,
};

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

async fn error(error_id: web::Path<String>) -> impl Responder {
    template::error().ok()
}
