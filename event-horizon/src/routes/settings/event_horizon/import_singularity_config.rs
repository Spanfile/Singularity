use std::sync::Arc;

use crate::{
    database::{DbPool, RedisPool},
    error::{EvhError, EvhResult},
    singularity::{RenderedConfig, SingularityConfig},
    template::{
        self,
        settings::{EventHorizonSubPage, SettingsPage},
        Alert, ResponseBuilder,
    },
    util::request_callback_error::RequestCallbackError,
    ConfigImporter,
};
use actix_multipart::Multipart;
use actix_web::{
    error::UrlencodedError,
    http::{header, StatusCode},
    web, Either, HttpRequest, HttpResponse, Responder,
};
use futures_util::{StreamExt, TryStreamExt};
use log::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TextImport {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ImportId {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ImportMergeForm {
    strategy: ImportMergeStrategy,
}

#[derive(Debug, Deserialize)]
enum ImportMergeStrategy {
    New,
    Merge,
    Overwrite,
    Cancel,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/import_singularity_config")
            .app_data(web::FormConfig::default().error_handler(import_form_error_handler))
            .route(web::get().to(import_singularity_config))
            .route(web::post().to(submit_import_form)),
    )
    .service(
        web::resource("/finish_config_import")
            .app_data(web::FormConfig::default().error_handler(finish_form_error_handler))
            .route(web::get().to(finish_config_import))
            .route(web::post().to(submit_finish_form)),
    );
}

fn import_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Import Singularity config POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        import_page()
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
            .render()
    })
    .into()
}

fn finish_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Finish config import POST failed: {}", err);
    warn!("{:?}", req);

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let importer = req
            .app_data::<web::Data<ConfigImporter>>()
            .expect("missing config importer");
        let redis_pool = req.app_data::<web::Data<RedisPool>>().expect("missing redis pool");
        let mut redis_conn = redis_pool.get().expect("failed to get redis connection");

        let import_id = web::Query::<ImportId>::from_query(req.query_string())
            .expect("failed to extract import ID parameter from query");
        let rendered_str = importer
            .get_blocking(&import_id.id, &mut *redis_conn)
            .and_then(|cfg| cfg.as_string())
            .expect("failed to get rendered config");

        finish_page(Some(rendered_str))
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
            .render()
    })
    .into()
}

async fn import_singularity_config() -> impl Responder {
    import_page()
}

async fn finish_config_import(
    import_id: web::Query<ImportId>,
    importer: web::Data<ConfigImporter>,
    redis_pool: web::Data<RedisPool>,
) -> impl Responder {
    match redis_pool
        .get()
        .map_err(EvhError::RedisConnectionAcquireFailed)
        // TODO: move blocking call to the thread pool
        .and_then(|mut redis_conn| importer.get_blocking(&import_id.id, &mut *redis_conn))
        .and_then(|cfg| cfg.as_string())
    {
        Ok(rendered) => finish_page(Some(rendered)),
        Err(e) => match e {
            EvhError::NoActiveImport(_) => {
                warn!("{}", e);

                finish_page(None)
                    .alert(Alert::Warning(format!("{}. Please retry the import.", e)))
                    .bad_request()
            }
            _ => {
                error!("Failed to render config finish page: {}", e);

                finish_page(None)
                    .alert(Alert::Error(format!("An internal server error occurred: {}", e)))
                    .internal_server_error()
            }
        },
    }
}

// TODO: this invokes the form error handler if the left side (the form) fails. make it not do that
async fn submit_import_form(
    payload: Either<web::Form<TextImport>, Multipart>,
    importer: web::Data<ConfigImporter>,
    redis_pool: web::Data<RedisPool>,
) -> impl Responder {
    match begin_import(payload, importer.into_inner(), redis_pool.into_inner()).await {
        Ok(import_id) => Either::Right(
            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((
                    header::LOCATION,
                    format!("/settings/event_horizon/finish_config_import?id={}", import_id),
                ))
                .finish(),
        ),
        Err(e) => match e {
            EvhError::UploadedFileNotUtf8 | EvhError::EmptyMultipartField | EvhError::MultipartError(_) => {
                Either::Left(import_page().alert(Alert::Error(e.to_string())).bad_request())
            }
            e => {
                error!("Beginning config import failed: {}", e);

                Either::Left(
                    import_page()
                        .alert(Alert::Error(format!("An internal error occurred: {}", e)))
                        .internal_server_error(),
                )
            }
        },
    }
}

async fn submit_finish_form(
    import_id: web::Query<ImportId>,
    merge_form: web::Form<ImportMergeForm>,
    importer: web::Data<ConfigImporter>,
    sing_cfg: web::Data<SingularityConfig>,
    db_pool: web::Data<DbPool>,
    redis_pool: web::Data<RedisPool>,
) -> impl Responder {
    info!(
        "Finishing Singularity config import {} with strategy: {:?}",
        import_id.id, merge_form.strategy
    );

    match finish_import(
        import_id.into_inner().id,
        merge_form.into_inner().strategy,
        importer.into_inner(),
        sing_cfg.into_inner(),
        db_pool.into_inner(),
        redis_pool.into_inner(),
    )
    .await
    {
        Ok(_) => {
            info!("Singularity config succesfully imported");

            Either::Right(
                HttpResponse::build(StatusCode::SEE_OTHER)
                    .append_header((header::LOCATION, "/settings/event_horizon"))
                    .finish(),
            )
        }
        Err(e) => match e {
            EvhError::NoActiveImport(id) => {
                warn!("No active import: {}", id);

                Either::Left(
                    finish_page(None)
                        .alert(Alert::Warning(format!(
                            "No active import with the ID {}. Please retry the import.",
                            id
                        )))
                        .bad_request(),
                )
            }
            e => {
                error!("Failed to finish importing Singularity config: {}", e);

                Either::Left(
                    finish_page(None)
                        .alert(Alert::Error(format!(
                            "Failed to finish importing Singularity config due to an internal error: {}",
                            e
                        )))
                        .internal_server_error(),
                )
            }
        },
    }
}

async fn begin_import(
    payload: Either<web::Form<TextImport>, Multipart>,
    importer: Arc<ConfigImporter>,
    redis_pool: Arc<RedisPool>,
) -> EvhResult<String> {
    let content = match payload {
        Either::Left(form) => {
            info!("Importing Singularity config from text");
            form.into_inner().text
        }
        Either::Right(mut payload) => {
            info!("Importing Singularity config from file");

            let mut field = payload
                .try_next()
                .await
                .map_err(|e| e.into())
                .and_then(|field| field.ok_or(EvhError::EmptyMultipartField))?;

            let mut buf = Vec::new();

            while let Some(chunk) = field.next().await {
                let data = chunk?;
                buf.extend_from_slice(&data);
            }

            debug!("File size: {}", buf.len());
            match String::from_utf8(buf) {
                Ok(content) => content,
                Err(_) => {
                    return Err(EvhError::UploadedFileNotUtf8);
                }
            }
        }
    };

    debug!("Received config:\n{}", content);

    let rendered = RenderedConfig::from_str(&content)?;
    debug!("Rendered: {:#?}", rendered);

    let import_id = web::block(move || {
        let mut redis_conn = redis_pool.get().map_err(EvhError::RedisConnectionAcquireFailed)?;
        importer.add_blocking(rendered, &mut *redis_conn)
    })
    .await
    .unwrap()?;

    debug!("Began config import with ID {}", import_id);
    Ok(import_id)
}

async fn finish_import(
    id: String,
    strategy: ImportMergeStrategy,
    importer: Arc<ConfigImporter>,
    sing_cfg: Arc<SingularityConfig>,
    db_pool: Arc<DbPool>,
    redis_pool: Arc<RedisPool>,
) -> EvhResult<()> {
    web::block(move || {
        let mut db_conn = db_pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
        let mut redis_conn = redis_pool.get().map_err(EvhError::RedisConnectionAcquireFailed)?;
        let rendered = importer.remove_blocking(&id, &mut *redis_conn)?;
        debug!("Using rendered config {}: {:#?}", id, rendered);

        match strategy {
            ImportMergeStrategy::New => {
                let new_config = SingularityConfig::new(&mut db_conn)?;
                new_config.overwrite(&mut db_conn, rendered)
            }
            ImportMergeStrategy::Merge => sing_cfg.merge(&mut db_conn, rendered),
            ImportMergeStrategy::Overwrite => sing_cfg.overwrite(&mut db_conn, rendered),
            ImportMergeStrategy::Cancel => Ok(()),
        }
    })
    .await
    .unwrap()?;

    Ok(())
}

fn import_page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::ImportSingularityConfig))
}

fn finish_page(rendered_cfg: Option<String>) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::FinishConfigImport(
        rendered_cfg,
    )))
}
