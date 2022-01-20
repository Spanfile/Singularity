mod add_new_adlist;
mod add_new_output;
mod add_whitelisted_domain;
mod delete_adlist;
mod delete_output;
mod delete_whitelisted_domain;

use crate::{
    database::DbPool,
    error::{EvhError, EvhResult},
    singularity::{AdlistCollection, OutputCollection, SingularityConfig, WhitelistCollection},
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
    },
};
use actix_web::{web, Responder};
use log::*;

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

async fn singularity(cfg: web::Data<SingularityConfig>, pool: web::Data<DbPool>) -> impl Responder {
    match page(&cfg, &pool) {
        Ok((adlists, outputs, whitelist)) => template::settings(SettingsPage::Singularity(SingularitySubPage::Main {
            adlists: &adlists,
            outputs: &outputs,
            whitelist: &whitelist,
        })),
        Err(e) => {
            error!("Failed to get main page: {}", e);
            todo!()
        }
    }
}

fn page(
    cfg: &SingularityConfig,
    pool: &DbPool,
) -> EvhResult<(AdlistCollection, OutputCollection, WhitelistCollection)> {
    let mut conn = pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;

    let adlists = cfg.adlists(&mut conn)?;
    let outputs = cfg.outputs(&mut conn)?;
    let whitelist = cfg.whitelist(&mut conn)?;

    Ok((adlists, outputs, whitelist))
}
