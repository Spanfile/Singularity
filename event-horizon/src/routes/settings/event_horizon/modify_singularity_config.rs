use crate::{
    database::{DbConn, DbId, DbPool},
    error::{EvhError, EvhResult},
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

#[derive(Debug, Deserialize)]
struct RenameForm {
    name: String,
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
        rename_config_page(None)
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
        delete_config_page(None)
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

    Either::Left(display_page(use_config_page, id, db_pool).await)
}

async fn rename_singularity_config_page(cfg_id: web::Query<ConfigId>, db_pool: web::Data<DbPool>) -> impl Responder {
    let id = cfg_id.id;
    display_page(rename_config_page, id, db_pool).await
}

async fn delete_singularity_config_page(
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

    Either::Left(display_page(delete_config_page, id, db_pool).await)
}

async fn form_action<FForm, FPage>(id: DbId, db_pool: web::Data<DbPool>, page: FPage, action: FForm) -> impl Responder
where
    FForm: FnOnce(&mut DbConn, SingularityConfig) -> EvhResult<()> + Send + 'static,
    FPage: Fn(Option<&str>) -> ResponseBuilder<'static>,
{
    let pool = db_pool.clone();
    match web::block(move || {
        // stupid hack: the name of the config may be required in the error handler to display the page properly, so
        // slap it in here with the error type if it's been loaded. guess this is one of those moments it's good Result
        // doesn't enforce its error type being Error huh?

        let mut conn = pool.get().map_err(|e| (None, e))?;
        let (name, cfg) = SingularityConfig::load(id, &mut conn).map_err(|e| (None, e))?;

        (action)(&mut conn, cfg).map_err(|e| (Some(name), e))
    })
    .await
    .unwrap()
    {
        Ok(_) => Either::Right(
            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/event_horizon"))
                .finish(),
        ),
        Err((name, e)) => match e {
            EvhError::EmptyConfigName | EvhError::DuplicateConfigName => {
                warn!(
                    "Failed to edit Singularity config ID {} due to an user error: {}",
                    id, e
                );

                Either::Left(
                    (page)(name.as_deref())
                        .alert(Alert::Warning(e.to_string()))
                        .bad_request(),
                )
            }
            e => {
                error!("Failed to edit Singularity config with ID {}: {}", id, e);

                Either::Left(
                    (page)(None)
                        .alert(Alert::Error(format!("An internal server error occurred: {}", e)))
                        .internal_server_error(),
                )
            }
        },
    }
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

    Either::Left(
        form_action(id, db_pool, use_config_page, move |conn, cfg| {
            cfg_mg.set_active_config(cfg);
            cfg.set_dirty(conn, true)
        })
        .await,
    )
}

async fn submit_rename_form(
    cfg_id: web::Query<ConfigId>,
    rename_form: web::Form<RenameForm>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = cfg_id.id;
    let name = rename_form.into_inner().name;
    info!("Renaming Singularity configuration with ID {} to {}", id, name);

    form_action(id, db_pool, rename_config_page, move |conn, cfg| {
        if name.trim().is_empty() {
            Err(EvhError::EmptyConfigName)
        } else if SingularityConfig::name_exists(conn, &name)? {
            Err(EvhError::DuplicateConfigName)
        } else {
            cfg.set_name(conn, &name)
        }
    })
    .await
}

async fn submit_delete_form(
    cfg_id: web::Query<ConfigId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = cfg_id.id;
    info!("Deleting Singularity configuration with ID {}", id);

    if cfg_mg.get_active_config().id() == id {
        warn!("Attempt to delete same config as already active one ({})", id);

        return Either::Right(
            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/event_horizon"))
                .finish(),
        );
    }

    Either::Left(form_action(id, db_pool, delete_config_page, move |conn, cfg| cfg.delete(conn)).await)
}

async fn display_page<F>(page_fn: F, id: DbId, db_pool: web::Data<DbPool>) -> impl Responder
where
    F: Fn(Option<&str>) -> ResponseBuilder<'static>,
{
    match web::block(move || {
        let mut conn = db_pool.get()?;
        SingularityConfig::load(id, &mut conn)
    })
    .await
    .unwrap()
    {
        Ok((name, _)) => Either::Left((page_fn)(Some(&name))),
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
                (page_fn)(None)
                    .alert(Alert::Error(format!(
                        "Failed to get Singularity config due to an internal server error: {}",
                        e
                    )))
                    .internal_server_error(),
            )
        }
    }
}

fn use_config_page(name: Option<&str>) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::UseSingularityConfig(
        name,
    )))
}

fn rename_config_page(name: Option<&str>) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(
        EventHorizonSubPage::RenameSingularityConfig(name),
    ))
}

fn delete_config_page(name: Option<&str>) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(
        EventHorizonSubPage::DeleteSingularityConfig(name),
    ))
}
