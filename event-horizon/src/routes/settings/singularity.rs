mod add_item;
mod delete_item;
mod set_timing;

use crate::{
    database::DbPool,
    error::EvhResult,
    singularity::singularity_config::{config_manager::ConfigManager, SingularityConfig},
    template::{
        self,
        settings::{SettingsPage, SingularityMainPageInformation, SingularitySubPage},
    },
    util,
};
use actix_web::{web, Either, Responder};
use std::sync::Arc;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/singularity")
            .route("", web::get().to(singularity))
            .configure(add_item::config)
            .configure(delete_item::config)
            .configure(set_timing::config),
    );
}

async fn singularity(cfg_mg: web::Data<ConfigManager>, pool: web::Data<DbPool>) -> impl Responder {
    let cfg = cfg_mg.get_active_config();
    match get_page_information(cfg, pool.into_inner()).await {
        Ok(page_info) => Either::Left(template::settings(SettingsPage::Singularity(SingularitySubPage::Main(
            page_info,
        )))),
        Err(e) => Either::Right(util::internal_server_error_response(format!(
            "Failed to get Singularity information: {}",
            e
        ))),
    }
}

async fn get_page_information(cfg: SingularityConfig, pool: Arc<DbPool>) -> EvhResult<SingularityMainPageInformation> {
    web::block(move || {
        let mut conn = pool.get()?;

        let cfg_name = cfg.get_name(&mut conn)?;
        let last_run = cfg.get_last_run(&mut conn)?;
        let timing = cfg.get_timing(&mut conn)?;
        let adlists = cfg.adlists(&mut conn)?;
        let outputs = cfg.outputs(&mut conn)?;
        let whitelist = cfg.whitelist(&mut conn)?;

        let next_run = util::next_cron_run(&timing)?;

        Ok(SingularityMainPageInformation {
            cfg_name,
            last_run,
            next_run,
            timing,
            adlists,
            outputs,
            whitelist,
        })
    })
    .await
    .expect("failed to spawn task in blocking thread pool")
}
