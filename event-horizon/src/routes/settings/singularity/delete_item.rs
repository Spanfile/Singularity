use crate::{
    database::{DbConn, DbId, DbPool},
    error::{EvhError, EvhResult},
    singularity::{ConfigManager, SingularityConfig},
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
        Alert, ResponseBuilder,
    },
    util::{self, request_callback_error::RequestCallbackError},
};
use actix_web::{
    error::UrlencodedError,
    http::{header, StatusCode},
    web, Either, HttpRequest, HttpResponse, Responder,
};
use log::*;
use serde::Deserialize;
use singularity::{Adlist, Output};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct DeleteId {
    id: DbId,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/delete_adlist")
            .app_data(web::FormConfig::default().error_handler(adlist_form_error_handler))
            .route(web::get().to(delete_adlist_page))
            .route(web::post().to(submit_adlist_form)),
    )
    .service(
        web::resource("/delete_output")
            .app_data(web::FormConfig::default().error_handler(output_form_error_handler))
            .route(web::get().to(delete_output_page))
            .route(web::post().to(submit_output_form)),
    )
    .service(
        web::resource("/delete_whitelisted_domain")
            .app_data(web::FormConfig::default().error_handler(whitelist_form_error_handler))
            .route(web::get().to(delete_whitelisted_domain_page))
            .route(web::post().to(submit_whitelist_form)),
    );
}

fn adlist_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(err, req, SingularityConfig::get_adlist, adlist_template)
}

fn output_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(err, req, SingularityConfig::get_output, output_template)
}

fn whitelist_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(err, req, SingularityConfig::get_whitelist, whitelist_template)
}

// common function to handle an invalid POSTed form for each route. item_fn is a function that retrieves a certain item
// T from the configuration, and template_fn is a function to create a page displaying it
fn form_error_handler<T, F, P>(err: UrlencodedError, req: &HttpRequest, item_fn: F, template_fn: P) -> actix_web::Error
where
    T: Send + 'static,
    F: Fn(&SingularityConfig, &mut DbConn, DbId) -> EvhResult<T> + Send + 'static,
    P: Fn(Option<(DbId, &T)>) -> ResponseBuilder<'static> + 'static,
{
    warn!("Delete item POST failed: {}", err);
    warn!("{:?}", req);

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let cfg_mg = req
            .app_data::<web::Data<ConfigManager>>()
            .ok_or(EvhError::MissingAppData)?;
        let pool = req.app_data::<web::Data<DbPool>>().ok_or(EvhError::MissingAppData)?;
        let mut conn = pool.get()?;

        let source = web::Query::<DeleteId>::from_query(req.query_string()).map_err(EvhError::InvalidQueryString)?;
        let cfg = cfg_mg.get_active_config();
        let item = (item_fn)(&cfg, &mut conn, source.id)?;

        Ok((template_fn)(Some((source.id, &item)))
            .alert(Alert::Warning(format!("Failed to delete item: {}", err)))
            .bad_request()
            .render())
    })
    .into()
}

async fn delete_adlist_page(
    id_query: web::Query<DeleteId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = id_query.id;
    display_page(
        id,
        db_pool.into_inner(),
        cfg_mg.get_active_config(),
        SingularityConfig::get_adlist,
        adlist_template,
    )
    .await
}

async fn delete_output_page(
    id_query: web::Query<DeleteId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = id_query.id;
    display_page(
        id,
        db_pool.into_inner(),
        cfg_mg.get_active_config(),
        |cfg, conn, id| {
            let (output, builtin) = cfg.get_output(conn, id)?;

            if builtin {
                Err(EvhError::AttemptToDeleteBuiltinOutput(id, output))
            } else {
                Ok((output, builtin))
            }
        },
        output_template,
    )
    .await
}

async fn delete_whitelisted_domain_page(
    id_query: web::Query<DeleteId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = id_query.id;
    display_page(
        id,
        db_pool.into_inner(),
        cfg_mg.get_active_config(),
        SingularityConfig::get_whitelist,
        whitelist_template,
    )
    .await
}

// common function to display a page for each route GET. item_fn is a function that retrieves a certain item T from the
// configuration, and template_fn is a function to create a page displaying it
async fn display_page<T, F, P>(
    id: DbId,
    db_pool: Arc<DbPool>,
    cfg: SingularityConfig,
    item_fn: F,
    template_fn: P,
) -> impl Responder
where
    T: Send + 'static,
    F: Fn(&SingularityConfig, &mut DbConn, DbId) -> EvhResult<T> + Send + 'static,
    P: Fn(Option<(DbId, &T)>) -> ResponseBuilder<'static>,
{
    match web::block(move || {
        let mut conn = db_pool.get()?;
        (item_fn)(&cfg, &mut conn, id)
    })
    .await
    .expect("failed to spawn task in blocking thread pool")
    {
        Ok(item) => Either::Left((template_fn)(Some((id, &item)))),
        Err(EvhError::NoSuchConfigItem(id)) => {
            warn!("Failed to retrieve item ID {}: not found", id);

            Either::Left(
                (template_fn)(None)
                    .alert(Alert::Warning(format!("Failed to retrieve item ID {}: not found", id)))
                    .bad_request(),
            )
        }

        // output deletion has a special case that it's not allowed to delete any builtin outputs, so handle that error
        // here instead of in the output function to keep the code short
        Err(EvhError::AttemptToDeleteBuiltinOutput(id, output)) => {
            warn!("Refusing to delete builtin output ID {}", id);

            Either::Left(
                // this is pretty terrible to call the output template directly with the builtin set to true but what
                // can you do
                output_template(Some((id, &(output, true))))
                    .alert(Alert::Warning(format!("Refusing to delete builtin output ID {}", id)))
                    .bad_request(),
            )
        }

        Err(e) => {
            error!("Failed to retrieve item ID {}: {}", id, e);
            Either::Right(util::internal_server_error_response(e))
        }
    }
}

async fn submit_adlist_form(
    id_form: web::Form<DeleteId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = id_form.id;
    let cfg = cfg_mg.get_active_config();
    submit_form(id, cfg, db_pool.into_inner(), SingularityConfig::delete_adlist).await
}

async fn submit_output_form(
    id_form: web::Form<DeleteId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = id_form.id;
    let cfg = cfg_mg.get_active_config();

    submit_form(id, cfg, db_pool.into_inner(), |cfg, conn, id| {
        let (output, builtin) = cfg.get_output(conn, id)?;

        if builtin {
            Err(EvhError::AttemptToDeleteBuiltinOutput(id, output))
        } else {
            cfg.delete_output(conn, id)
        }
    })
    .await
}

async fn submit_whitelist_form(
    id_form: web::Form<DeleteId>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    let id = id_form.id;
    let cfg = cfg_mg.get_active_config();
    submit_form(
        id,
        cfg,
        db_pool.into_inner(),
        SingularityConfig::delete_whitelisted_domain,
    )
    .await
}

// common function to handle the POSTed forms for each route. delete_fn is a function that deletes the certain item T
// from the configuration
async fn submit_form<F>(id: DbId, cfg: SingularityConfig, db_pool: Arc<DbPool>, delete_fn: F) -> impl Responder
where
    F: Fn(&SingularityConfig, &mut DbConn, DbId) -> EvhResult<()> + Send + 'static,
{
    info!("Deleting item: {}", id);

    match web::block(move || {
        let mut conn = db_pool.get()?;
        (delete_fn)(&cfg, &mut conn, id)
    })
    .await
    .expect("failed to spawn task in blocking thread pool")
    {
        Ok(()) => {
            info!("Item ID {} succesfully deleted", id);

            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/settings/singularity"))
                .finish()
        }
        Err(EvhError::NoSuchConfigItem(id)) => {
            warn!("Failed to delete item ID {}: not found", id);

            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/settings/singularity"))
                .finish()
        }

        // output deletion has a special case that it's not allowed to delete any builtin outputs, so handle that error
        // here instead of in the output function to keep the code short
        Err(EvhError::AttemptToDeleteBuiltinOutput(id, _)) => {
            warn!("Attempt to delete builtin output ID {}", id);

            // TODO: maybe the display the output template page here?
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/settings/singularity"))
                .finish()
        }

        Err(e) => {
            error!("Failed to delete item: {}", e);
            util::internal_server_error_response(e)
        }
    }
}

fn adlist_template(id_adlist: Option<(DbId, &Adlist)>) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::DeleteAdlist(id_adlist)))
}

fn output_template(id_output: Option<(DbId, &(Output, bool))>) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::DeleteOutput(
        id_output.map(|(id, (output, builtin))| (id, output, *builtin)),
    )))
}

fn whitelist_template(id_whitelist: Option<(DbId, &String)>) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::DeleteWhitelistedDomain(
        id_whitelist.map(|(id, domain)| (id, domain.as_str())),
    )))
}
