mod event_horizon;
mod recursor;
mod singularity;

use actix_web::{http::header, web, HttpResponse, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/settings")
            .route("", web::get().to(settings))
            .configure(singularity::config)
            .configure(event_horizon::config)
            .configure(recursor::config),
    );
}

async fn settings() -> impl Responder {
    HttpResponse::MovedPermanently()
        .customize()
        .insert_header((header::LOCATION, "/settings/event_horizon"))
}
