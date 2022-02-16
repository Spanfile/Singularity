use crate::{
    config::EvhConfig,
    database::DbPool,
    error::EvhError,
    singularity::{
        runner::{CurrentlyRunningSingularity, SingularityRunner},
        singularity_config::config_manager::ConfigManager,
    },
    template, util,
};
use actix_web::{http::header, web, Either, HttpResponse, Responder};
use log::*;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/run")
            .route(web::get().to(run_singularity_page))
            .route(web::post().to(submit_run_singularity_form)),
    );
}

async fn run_singularity_page(
    runner: web::Data<SingularityRunner>,
    evh_cfg: web::Data<EvhConfig>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    match runner.get_currently_running().await {
        None => Either::Right(
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/singularity"))
                .finish(),
        ),
        Some(CurrentlyRunningSingularity::Running) => Either::Left(template::singularity::singularity_running()),
        Some(CurrentlyRunningSingularity::Finished) => {
            match db_pool
                .get()
                .and_then(|mut conn| runner.get_finished_history(&mut conn, &evh_cfg))
            {
                Ok(history) => Either::Left(template::singularity::singularity_finished(
                    history.timestamp(),
                    history.events(),
                )),

                Err(EvhError::NoSuchHistory(_)) => todo!(),
                Err(EvhError::NoPreviousRun) => todo!(),
                Err(e) => Either::Right(util::internal_server_error_response(format!(
                    "Failed to get previous run history: {}",
                    e
                ))),
            }
        }
    }
}

async fn submit_run_singularity_form(
    runner: web::Data<SingularityRunner>,
    cfg_mg: web::Data<ConfigManager>,
    evh_cfg: web::Data<EvhConfig>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let cfg = cfg_mg.get_active_config();

    match runner.run(cfg, pool.into_inner(), evh_cfg.into_inner()) {
        Ok(_) => HttpResponse::SeeOther()
            .append_header((header::LOCATION, "/singularity/run"))
            .finish(),
        Err(EvhError::SingularityRunning) => {
            warn!("Failed to run Singularity: already running");
            HttpResponse::BadRequest().body("Singularity is already running")
        }
        Err(e) => util::internal_server_error_response(format!("Failed to run Singularity: {}", e)),
    }
}
