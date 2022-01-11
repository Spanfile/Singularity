use crate::template::{self, settings::SettingsPage};
use actix_web::{web, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/event_horizon").route("", web::get().to(event_horizon)));
}

async fn event_horizon() -> impl Responder {
    template::settings(SettingsPage::EventHorizon).ok()
}
