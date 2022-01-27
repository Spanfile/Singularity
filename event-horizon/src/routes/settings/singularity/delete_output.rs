use crate::{
    database::{DbId, DbPool},
    error::{EvhError, EvhResult},
    singularity::{ConfigManager, SingularityConfig},
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
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct DeleteId {
    id: DbId,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/delete_output")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(delete_output_page))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Delete output POST failed: {}", err);
    warn!("{:?}", req);

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let cfg = req
            .app_data::<web::Data<ConfigManager>>()
            .expect("missing singularity config");
        let pool = req.app_data::<web::Data<DbPool>>().expect("missing DB pool");

        let source = web::Query::<DeleteId>::from_query(req.query_string())
            .expect("failed to extract source parameter from query");

        page_blocking(source.id, cfg.get_active_config(), pool)
            .alert(Alert::Warning(format!("Failed to delete output: {}", err)))
            .bad_request()
            .render()
    })
    .into()
}

async fn delete_output_page(
    id: web::Query<DeleteId>,
    cfg_mg: web::Data<ConfigManager>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    page(id.id, cfg_mg.get_active_config(), pool.into_inner()).await
}

async fn submit_form(
    id: web::Form<DeleteId>,
    cfg_mg: web::Data<ConfigManager>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let pool = pool.into_inner();
    let id = id.into_inner().id;
    info!("Deleting output: {}", id);

    match delete(id, cfg_mg.get_active_config(), Arc::clone(&pool)).await {
        Ok(_) => {
            info!("Output succesfully deleted");

            Either::Right(
                HttpResponse::build(StatusCode::SEE_OTHER)
                    .append_header((header::LOCATION, "/settings/singularity"))
                    .finish(),
            )
        }
        Err(e) => match e {
            EvhError::Database(diesel::result::Error::NotFound) => {
                warn!("Failed to delete output: output not found");
                warn!("{}", e);

                Either::Left(
                    page(id, cfg_mg.get_active_config(), pool)
                        .await
                        .alert(Alert::Warning("Output to delete was not found".to_string()))
                        .bad_request(),
                )
            }
            _ => {
                error!("Failed to delete output: {}", e);

                Either::Left(
                    page(id, cfg_mg.get_active_config(), pool)
                        .await
                        .alert(Alert::Warning(format!(
                            "Failed to delete output due to an internal server error: {}",
                            e
                        )))
                        .internal_server_error(),
                )
            }
        },
    }
}

async fn delete(id: DbId, cfg: SingularityConfig, pool: Arc<DbPool>) -> EvhResult<()> {
    web::block(move || {
        let mut conn = pool.get()?;
        cfg.delete_output(&mut conn, id)
    })
    .await
    .unwrap()?;

    Ok(())
}

fn page_blocking<'a>(id: DbId, cfg: SingularityConfig, pool: &DbPool) -> ResponseBuilder<'a> {
    let mut conn = pool.get().unwrap();
    let output = cfg.get_output(&mut conn, id).expect("failed to get output");
    template::settings(SettingsPage::Singularity(SingularitySubPage::DeleteOutput(id, &output)))
}

async fn page<'a>(id: DbId, cfg: SingularityConfig, pool: Arc<DbPool>) -> ResponseBuilder<'a> {
    web::block(move || page_blocking(id, cfg, &pool)).await.unwrap()
}
