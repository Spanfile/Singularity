use crate::{
    database::DbPool,
    error::{EvhError, EvhResult},
    singularity::{ConfigManager, SingularityConfig},
    util::{self, request_callback_error::RequestCallbackError},
};
use actix_web::{
    error::UrlencodedError,
    http::{header, StatusCode},
    web, HttpRequest, HttpResponse, Responder,
};
use cron_clock::Schedule;
use log::*;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct TimingForm {
    expression: String,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/set_timing")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(redirect_to_settings))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Set Singularity timing schedule POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        HttpResponse::build(StatusCode::SEE_OTHER)
            .append_header((header::LOCATION, "/settings/singularity"))
            .finish()
    })
    .into()
}

async fn redirect_to_settings() -> impl Responder {
    HttpResponse::build(StatusCode::SEE_OTHER)
        .append_header((header::LOCATION, "/settings/singularity"))
        .finish()
}

async fn submit_form(
    timing_form: web::Form<TimingForm>,
    cfg_mg: web::Data<ConfigManager>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    info!("Setting Singularity timing schedule: {:?}", timing_form);

    match do_set_schedule(timing_form.into_inner(), cfg_mg.get_active_config(), pool.into_inner()).await {
        Ok(()) => {
            info!("Timing schedule succesfully set");
        }

        // TODO: these really gotta display an error alert in the page somehow
        Err(EvhError::InvalidCronSchedule(e)) => {
            warn!("Failed to set Singularity timing schedule: {}", e);
        }
        Err(e) => {
            error!("Failed to set Singularity timing schedule: {}", e);
        }
    }

    HttpResponse::build(StatusCode::SEE_OTHER)
        .append_header((header::LOCATION, "/settings/singularity"))
        .finish()
}

async fn do_set_schedule(timing: TimingForm, cfg: SingularityConfig, pool: Arc<DbPool>) -> EvhResult<()> {
    // the Schedule object cannot be directly stored in the database (the source string is stored instead), so this step
    // is only to validate the cron spec

    let expression = util::expand_cron_expression(&timing.expression);
    let _: Schedule = expression.parse().map_err(EvhError::InvalidCronSchedule)?;

    web::block(move || {
        let mut conn = pool.get()?;
        cfg.set_timing(&mut conn, &timing.expression)
    })
    .await
    .unwrap()
}
