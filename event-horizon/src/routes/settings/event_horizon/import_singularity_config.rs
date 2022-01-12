use crate::{
    config::EvhConfig,
    database::DbPool,
    error::{EvhError, EvhResult},
    singularity::{ConfigImporter, RenderedConfig, SingularityConfig},
    template::{
        self,
        settings::{EventHorizonSubPage, SettingsPage},
        Alert, ResponseBuilder,
    },
    util::request_callback_error::RequestCallbackError,
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
use std::sync::RwLock;

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
        import_page().alert(Alert::Warning(err.to_string())).bad_request()
    })
    .into()
}

fn finish_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Finish config import POST failed: {}", err);
    warn!("{:?}", req);

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let importer = req
            .app_data::<web::Data<RwLock<ConfigImporter>>>()
            .and_then(|importer| importer.read().ok())
            .expect("config importer rwlock is poisoned");
        let import_id = web::Query::<ImportId>::from_query(req.query_string())
            .expect("failed to extract import ID parameter from query");
        let rendered_str = importer
            .get_string(&import_id.id)
            .expect("failed to get rendered config");

        finish_page(&rendered_str)
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
    })
    .into()
}

async fn import_singularity_config() -> impl Responder {
    import_page().ok()
}

async fn finish_config_import(
    import_id: web::Query<ImportId>,
    importer: web::Data<RwLock<ConfigImporter>>,
) -> impl Responder {
    let importer = importer.read().expect("config importer rwlock is poisoned");

    match importer.get_string(&import_id.id) {
        Ok(rendered_str) => finish_page(&rendered_str).ok(),
        Err(_) => todo!("DO SOME GOOD ERROR HANDLING OKAY?"),
    }
}

// TODO: this invokes the form error handler if the left side (the form) fails. make it not do that
async fn submit_import_form(
    payload: Either<web::Form<TextImport>, Multipart>,
    importer: web::Data<RwLock<ConfigImporter>>,
    evh_config: web::Data<EvhConfig>,
) -> impl Responder {
    match begin_import(payload, &importer, &evh_config).await {
        Ok(import_id) => HttpResponse::build(StatusCode::SEE_OTHER)
            .append_header((
                header::LOCATION,
                format!("/settings/event_horizon/finish_config_import?id={}", import_id),
            ))
            .finish(),
        Err(e) => match e {
            EvhError::UploadedFileNotUtf8 | EvhError::EmptyMultipartField | EvhError::MultipartError(_) => {
                import_page().alert(Alert::Error(e.to_string())).bad_request()
            }
            _ => import_page()
                .alert(Alert::Error(format!("An internal error occurred: {}", e)))
                .internal_server_error(),
        },
    }
}

async fn submit_finish_form(
    import_id: web::Query<ImportId>,
    merge_form: web::Form<ImportMergeForm>,
    importer: web::Data<RwLock<ConfigImporter>>,
    sing_cfg: web::Data<SingularityConfig>,
    evh_config: web::Data<EvhConfig>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    match finish_import(
        import_id.into_inner().id,
        merge_form.into_inner().strategy,
        &importer,
        &sing_cfg,
        &evh_config,
        &pool,
    ) {
        Ok(_) => HttpResponse::build(StatusCode::SEE_OTHER)
            .append_header((header::LOCATION, "/settings/event_horizon"))
            .finish(),
        Err(e) => import_page()
            .alert(Alert::Error(format!("An internal error occurred: {}", e)))
            .internal_server_error(),
    }
}

async fn begin_import(
    payload: Either<web::Form<TextImport>, Multipart>,
    importer: &RwLock<ConfigImporter>,
    evh_config: &EvhConfig,
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

    let import_id = importer
        .write()
        .expect("importer rw lock is poisoned")
        .begin_import(rendered, evh_config);

    debug!("Began config import with ID {}", import_id);
    Ok(import_id)
}

fn finish_import(
    id: String,
    strategy: ImportMergeStrategy,
    importer: &RwLock<ConfigImporter>,
    sing_cfg: &SingularityConfig,
    evh_config: &EvhConfig,
    pool: &DbPool,
) -> EvhResult<()> {
    info!(
        "Finishing Singularity config import {} with strategy: {:?}",
        id, strategy
    );

    let mut importer = importer.write().expect("importer rw lock is poisoned");
    let mut conn = pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
    let rendered = importer.finish(&id, evh_config)?;

    debug!("Using rendered config {}: {:#?}", id, rendered);

    match strategy {
        ImportMergeStrategy::New => {
            let new_config = SingularityConfig::new(&mut conn)?;
            new_config.overwrite(&mut conn, rendered)?;
        }
        ImportMergeStrategy::Merge => {
            todo!()
        }
        ImportMergeStrategy::Overwrite => sing_cfg.overwrite(&mut conn, rendered)?,
        ImportMergeStrategy::Cancel => importer.cancel_import(&id, evh_config),
    }

    Ok(())
}

fn import_page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::ImportSingularityConfig))
}

fn finish_page(rendered_str: &str) -> ResponseBuilder {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::FinishConfigImport(
        rendered_str,
    )))
}
