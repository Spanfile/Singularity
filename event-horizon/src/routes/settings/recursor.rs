use crate::template::{self, settings::SettingsPage};
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/recursor").route("", web::get().to(recursor)));
}

async fn recursor() -> impl Responder {
    template::settings(SettingsPage::Recursor)
}
