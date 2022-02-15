use actix_web::{web, HttpResponse, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/get").route(web::get().to(history_page)));
}

async fn history_page() -> impl Responder {
    HttpResponse::NotFound()
}
