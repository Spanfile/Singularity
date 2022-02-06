use crate::{database::RedisVersion, rec_control::RecursorVersion, template};
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/about").route(web::get().to(about)));
}

async fn about(recursor_version: web::Data<RecursorVersion>, redis_version: web::Data<RedisVersion>) -> impl Responder {
    template::about(&recursor_version.0, &redis_version.0).current_path("/about")
}
