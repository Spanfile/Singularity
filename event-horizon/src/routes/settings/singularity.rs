mod add_new_adlist;
mod add_new_output;
mod add_whitelisted_domain;
mod delete_adlist;
mod delete_output;
mod delete_whitelisted_domain;

use crate::{
    database::DbPool,
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
            .route("", web::get().to(singularity))
            .configure(add_new_adlist::config)
            .configure(delete_adlist::config)
            .configure(add_new_output::config)
            .configure(delete_output::config)
            .configure(add_whitelisted_domain::config)
            .configure(delete_whitelisted_domain::config),
    );
}

async fn singularity(cfg: web::Data<RwLock<SingularityConfig>>, pool: web::Data<DbPool>) -> impl Responder {
    let cfg = cfg.read().expect("failed to lock read singularity config");
    let mut conn = pool.get().expect("failed to get DB connection");

    let adlists = cfg.adlists(&mut conn).expect("failed to read adlists");
    let outputs = cfg.outputs(&mut conn).expect("failed to read outputs");
    let whitelist = cfg.whitelist(&mut conn).expect("failed to read whitelist");

    template::settings(SettingsPage::Singularity(SingularitySubPage::Main {
        adlists: &adlists,
        outputs: &outputs,
        whitelist: &whitelist,
    }))
    .ok()
}
