use crate::{
    database::DbPool,
    error::EvhResult,
    singularity::singularity_config::{config_manager::ConfigManager, SingularityConfig},
    template, util,
};
use actix_web::{http::header, web, Either, HttpResponse, Responder};
use chrono::{DateTime, Local};
use log::*;
use nanoid::nanoid;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct RunQuery {
    run: Option<String>,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/run_singularity")
            .route(web::get().to(run_singularity_page))
            .route(web::post().to(submit_run_singularity_form)),
    );
}

async fn run_singularity_page(
    run_query: web::Query<RunQuery>,
    cfg_mg: web::Data<ConfigManager>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if let Some(run) = run_query.run.as_deref() {
        // TODO: check the run, if it's pending show the singularity_running() page, if it's done show the done page

        Either::Left(template::run_singularity::singularity_running())
    } else {
        Either::Right(template_run_singularity(cfg_mg, pool).await)
    }
}

async fn submit_run_singularity_form() -> impl Responder {
    // trigger the run here
    let run_id = nanoid!();

    HttpResponse::SeeOther()
        .append_header((header::LOCATION, format!("/run_singularity?run={}", run_id)))
        .finish()
}

async fn template_run_singularity(cfg_mg: web::Data<ConfigManager>, pool: web::Data<DbPool>) -> impl Responder {
    // TODO: display currently running if any, and history of runs

    let cfg = cfg_mg.get_active_config();
    match get_singularity_run_times(cfg, pool.into_inner()).await {
        Ok((last_run, next_run)) => Either::Left(template::run_singularity(last_run, next_run)),
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
