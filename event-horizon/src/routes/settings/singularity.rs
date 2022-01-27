// TODO: all of these routes are more-or-less identical, but sorta complex, so it'd be nice to refactor them into one
// common implementation
mod add_new_adlist;
mod add_new_output;
mod add_whitelisted_domain;
mod delete_adlist;
mod delete_output;
mod delete_whitelisted_domain;

use crate::{
    database::DbPool,
    error::EvhResult,
    singularity::{AdlistCollection, ConfigManager, OutputCollection, SingularityConfig, WhitelistCollection},
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
    },
};
use actix_web::{web, Responder};
use log::*;
use std::sync::Arc;

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

async fn singularity(cfg_mg: web::Data<ConfigManager>, pool: web::Data<DbPool>) -> impl Responder {
    match page(cfg_mg.get_active_config(), pool.into_inner()).await {
        Ok((name, adlists, outputs, whitelist)) => {
            template::settings(SettingsPage::Singularity(SingularitySubPage::Main {
                cfg_name: &name,
                adlists: &adlists,
                outputs: &outputs,
                whitelist: &whitelist,
            }))
        }
        Err(e) => {
            error!("Failed to get main page: {}", e);
            todo!()
        }
    }
}

async fn page(
    cfg: SingularityConfig,
    pool: Arc<DbPool>,
) -> EvhResult<(String, AdlistCollection, OutputCollection, WhitelistCollection)> {
    web::block(move || {
        let mut conn = pool.get()?;

        let name = cfg.get_name(&mut conn)?;
        let adlists = cfg.adlists(&mut conn)?;
        let outputs = cfg.outputs(&mut conn)?;
        let whitelist = cfg.whitelist(&mut conn)?;

        Ok((name, adlists, outputs, whitelist))
    })
    .await
    .unwrap()
}
