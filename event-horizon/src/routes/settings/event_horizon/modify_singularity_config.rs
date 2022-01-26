use crate::{
    database::{DbId, DbPool},
    error::EvhError,
    singularity::{ConfigManager, SingularityConfig},
    template::{
        self,
        settings::{EventHorizonSubPage, SettingsPage},
        Alert, ResponseBuilder,
    },
    util::request_callback_error::RequestCallbackError,
};
use actix_web::{
    error::UrlencodedError,
    http::{header, StatusCode},
    web, Either, HttpRequest, HttpResponse, Responder,
};
use log::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ConfigId {
    id: DbId,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/use_singularity_config")
            .app_data(web::FormConfig::default().error_handler(use_form_error_handler))
            .route(web::get().to(use_singularity_config_page))
            .route(web::post().to(submit_use_form)),
    )
    .service(
        web::resource("/rename_singularity_config")
            .app_data(web::FormConfig::default().error_handler(rename_form_error_handler))
            .route(web::get().to(rename_singularity_config_page))
            .route(web::post().to(submit_rename_form)),
    )
    .service(
        web::resource("/delete_singularity_config")
            .app_data(web::FormConfig::default().error_handler(delete_form_error_handler))
            .route(web::get().to(delete_singularity_config_page))
            .route(web::post().to(submit_delete_form)),
    );
}

fn use_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Use Singularity config POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        use_config_page(None)
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
            .render()
    })
    .into()
}

fn rename_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Rename Singularity config POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        rename_config_page()
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
            .render()
    })
    .into()
}

fn delete_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Delete Singularity config POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        delete_config_page()
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
            .render()
    })
    .into()
}

async fn use_singularity_config_page(
    cfg_id: web::Query<ConfigId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = cfg_id.id;

    if cfg_mg.get_active_config().id() == id {
        warn!("Attempt to use same config as already active one ({})", id);

        return Either::Right(
            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/event_horizon"))
                .finish(),
        );
    }

    match web::block(move || {
        let mut conn = db_pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
        SingularityConfig::load(id, &mut conn)
    })
    .await
    .unwrap()
    {
        Ok((name, _)) => Either::Left(use_config_page(Some(&name))),
        Err(EvhError::NoSuchConfig(id)) => {
            warn!("No such Singularity config with ID {}", id);

            Either::Right(
                HttpResponse::build(StatusCode::SEE_OTHER)
                    .append_header((header::LOCATION, "/settings/event_horizon"))
                    .finish(),
            )
        }
        Err(e) => {
            error!("Failed to get Singularity config with ID {}: {}", id, e);

            Either::Left(
                use_config_page(None)
                    .alert(Alert::Error(format!(
                        "Failed to get Singularity config due to an internal server error: {}",
                        e
                    )))
                    .internal_server_error(),
            )
        }
    }
}

async fn rename_singularity_config_page(cfg_id: web::Query<ConfigId>) -> impl Responder {
    rename_config_page()
}

async fn delete_singularity_config_page(cfg_id: web::Query<ConfigId>) -> impl Responder {
    delete_config_page()
}

async fn submit_use_form(
    cfg_id: web::Query<ConfigId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = cfg_id.id;

    info!("Setting active Singularity configuration to ID {}", id);

    if cfg_mg.get_active_config().id() == id {
        warn!("Attempt to use same config as already active one ({})", id);

        return Either::Right(
            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/event_horizon"))
                .finish(),
        );
    }

    let pool = db_pool.clone();
    match web::block(move || {
        let mut conn = pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
        let (_, cfg) = SingularityConfig::load(id, &mut conn)?;

        info!("Setting current active Singularity config to {:?}", cfg);
        cfg_mg.set_active_config(cfg);
        cfg.set_dirty(&mut conn, true)
    })
    .await
    .unwrap()
    {
        Ok(_) => Either::Right(
            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/event_horizon"))
                .finish(),
        ),

        Err(EvhError::NoSuchConfig(id)) => {
            warn!("No such Singularity config with ID {}", id);

            Either::Left(
                use_config_page(None)
                    .alert(Alert::Warning(format!(
                        "Failed to set active Singularity configuration: ID {} not found",
                        id
                    )))
                    .bad_request(),
            )
        }
        Err(e) => {
            error!("Failed to set Singularity config with ID {}: {}", id, e);

            Either::Left(
                use_config_page(None)
                    .alert(Alert::Error(format!(
                        "Failed to set active Singularity configuration due to an internal server error: {}",
                        e
                    )))
                    .internal_server_error(),
            )
        }
    }
}

async fn submit_rename_form(cfg_id: web::Query<ConfigId>) -> impl Responder {
    ""
}

async fn submit_delete_form(cfg_id: web::Query<ConfigId>) -> impl Responder {
    ""
}

fn use_config_page(name: Option<&str>) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::UseSingularityConfig(
        name,
    )))
}

fn rename_config_page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::RenameSingularityConfig))
}

fn delete_config_page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::DeleteSingularityConfig))
}
