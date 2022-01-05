mod add_new_adlist;
mod remove_adlist;

use crate::{
    singularity::SingularityConfig,
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
    },
};
use actix_web::{web, Responder};
use std::sync::RwLock;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/singularity")
            .service(web::resource("").route(web::get().to(singularity)))
            .configure(add_new_adlist::config)
            .configure(remove_adlist::config),
    );
}

async fn singularity(singularity_config: web::Data<RwLock<SingularityConfig>>) -> impl Responder {
    let cfg = singularity_config
        .read()
        .expect("failed to lock read singularity config");

    template::settings(SettingsPage::Singularity(SingularitySubPage::Main), &cfg).ok()
}
