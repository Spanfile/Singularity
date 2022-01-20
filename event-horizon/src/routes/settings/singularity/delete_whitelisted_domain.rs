use crate::{
    database::{DbId, DbPool},
    error::{EvhError, EvhResult},
    singularity::SingularityConfig,
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
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
struct DeleteId {
    id: DbId,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/delete_whitelisted_domain")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(delete_whitelisted_domain))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Delete whitelisted domain POST failed: {}", err);
    warn!("{:?}", req);

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let cfg = req
            .app_data::<web::Data<SingularityConfig>>()
            .expect("missing singularity config");
        let pool = req.app_data::<web::Data<DbPool>>().expect("missing DB pool");

        let source = web::Query::<DeleteId>::from_query(req.query_string())
            .expect("failed to extract source parameter from query");

        page(source.id, cfg, pool)
            .alert(Alert::Warning(format!("Failed to delete whitelisted domain: {}", err)))
            .bad_request()
            .render()
    })
    .into()
}

async fn delete_whitelisted_domain(
    id: web::Query<DeleteId>,
    cfg: web::Data<SingularityConfig>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    page(id.id, &cfg, &pool)
}

async fn submit_form(
    id: web::Form<DeleteId>,
    cfg: web::Data<SingularityConfig>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let id = id.into_inner().id;
    info!("Deleting whitelisted domain: {}", id);

    match delete(id, &cfg, &pool) {
        Ok(_) => {
            info!("Whitelisted domain succesfully deleted");

            Either::Left(
                HttpResponse::build(StatusCode::SEE_OTHER)
                    .append_header((header::LOCATION, "/settings/singularity"))
                    .finish(),
            )
        }
        Err(e) => match e {
            EvhError::Database(diesel::result::Error::NotFound) => {
                warn!("Failed to delete whitelisted domain: whitelisted domain not found");
                warn!("{}", e);

                Either::Right(
                    page(id, &cfg, &pool)
                        .alert(Alert::Warning("Whitelisted domain to delete was not found".to_string()))
                        .bad_request(),
                )
            }
            _ => {
                error!("Failed to delete output: {}", e);

                Either::Right(
                    page(id, &cfg, &pool)
                        .alert(Alert::Warning(format!(
                            "Failed to delete whitelisted domain due to an internal server error: {}",
                            e
                        )))
                        .internal_server_error(),
                )
            }
        },
    }
}

fn delete(id: DbId, cfg: &SingularityConfig, pool: &DbPool) -> EvhResult<()> {
    let mut conn = pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
    cfg.delete_whitelisted_domain(&mut conn, id)?;
    Ok(())
}

fn page<'a>(id: DbId, cfg: &SingularityConfig, pool: &DbPool) -> ResponseBuilder<'a> {
    let mut conn = pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed).unwrap();
    let domain = cfg
        .get_whitelist(&mut conn, id)
        .expect("failed to get whitelisted domain");

    template::settings(SettingsPage::Singularity(SingularitySubPage::DeleteWhitelistedDomain(
        id, &domain,
    )))
}
