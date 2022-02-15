mod history;
mod run;

use crate::{
    database::DbPool,
    error::EvhResult,
    singularity::{
        singularity_config::{config_manager::ConfigManager, SingularityConfig},
        singularity_runner::{CurrentlyRunningSingularity, SingularityRunner},
    },
    template, util,
};
use actix_web::{web, Either, Responder};
use chrono::{DateTime, Local};
use log::*;
use std::sync::Arc;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/singularity")
            .configure(run::config)
            .configure(history::config)
            .route("", web::get().to(singularity_page)),
    );
}

async fn singularity_page(
    cfg_mg: web::Data<ConfigManager>,
    runner: web::Data<SingularityRunner>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    // TODO: display currently running if any, and history of runs

    let cfg = cfg_mg.get_active_config();
    match get_singularity_run_times(cfg, pool.into_inner()).await {
        Ok((last_run, next_run)) => {
            let currently_running = matches!(
                runner.get_currently_running().await,
                Some(CurrentlyRunningSingularity::Running)
            );

            Either::Left(template::singularity_page(last_run, next_run, currently_running))
        }
        Err(e) => {
            error!("Failed to get Singularity run page: {}", e);
            Either::Right(util::internal_server_error_response(e.to_string()))
        }
    }
}

async fn get_singularity_run_times(
    cfg: SingularityConfig,
    pool: Arc<DbPool>,
) -> EvhResult<(Option<DateTime<Local>>, DateTime<Local>)> {
    web::block(move || {
        let mut conn = pool.get()?;

        let last_run = cfg.get_last_run(&mut conn)?;
        let timing = cfg.get_timing(&mut conn)?;
        let next_run = util::next_cron_run(&timing)?;

        Ok((last_run, next_run))
    })
    .await
    .expect("failed to spawn task in blocking thread pool")
}
