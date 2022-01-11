use std::sync::RwLock;

use crate::{
    config::EvhConfig,
    singularity::ConfigImporter,
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

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        finish_page().alert(Alert::Warning(err.to_string())).bad_request()
    })
    .into()
}

async fn import_singularity_config() -> impl Responder {
    import_page().ok()
}

async fn finish_config_import() -> impl Responder {
    finish_page().ok()
}

// TODO: this invokes the form error handler if the left side (the form) fails. make it not do that
async fn submit_import_form(
    payload: Either<web::Form<TextImport>, Multipart>,
    importer: web::Data<RwLock<ConfigImporter>>,
    evh_config: web::Data<EvhConfig>,
) -> impl Responder {
    let content = match payload {
        Either::Left(form) => {
            info!("Importing Singularity config from text");
            form.into_inner().text
        }
        Either::Right(mut payload) => {
            info!("Importing Singularity config from file");

            match payload
                .try_next()
                .await
                .map_err(|e| e.into())
                .and_then(|field| field.ok_or(anyhow::anyhow!("empty form")))
            {
                Ok(mut field) => {
                    let mut buf = Vec::new();

                    while let Some(chunk) = field.next().await {
                        let data = chunk.expect("failed to read chunk");
                        buf.extend_from_slice(&data);
                    }

                    debug!("File size: {}", buf.len());
                    match String::from_utf8(buf) {
                        Ok(content) => content,
                        Err(_) => {
                            return import_page()
                                .alert(Alert::Warning(
                                    "Reading file failed: file is not encoded in UTF-8".to_string(),
                                ))
                                .bad_request();
                        }
                    }
                }
                Err(e) => {
                    return import_page()
                        .alert(Alert::Error(format!("Reading file failed: {:?}", e)))
                        .internal_server_error();
                }
            }
        }
    };

    debug!("Received config:\n{}", content);

    let import_id = importer
        .write()
        .expect("failed to lock write config importer")
        .begin_import(content, &evh_config)
        .expect("failed to begin config import");

    debug!("Began config import with ID {}", import_id);

    HttpResponse::build(StatusCode::SEE_OTHER)
        .append_header((
            header::LOCATION,
            format!("/settings/event_horizon/finish_config_import?id={}", import_id),
        ))
        .finish()
}

async fn submit_finish_form(
    import_id: web::Query<ImportId>,
    merge_form: web::Form<ImportMergeForm>,
    importer: web::Data<RwLock<ConfigImporter>>,
    evh_config: web::Data<EvhConfig>,
) -> impl Responder {
    info!(
        "Finishing Singularity config import {} with strategy: {:?}",
        import_id.id, merge_form
    );

    let import_id = import_id.into_inner();
    let mut importer = importer.write().expect("failed to lock write config importer");

    match merge_form.strategy {
        ImportMergeStrategy::New => todo!(),
        ImportMergeStrategy::Merge => todo!(),
        ImportMergeStrategy::Overwrite => todo!(),
        ImportMergeStrategy::Cancel => {
            importer.cancel_import(&import_id.id, &evh_config);

            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/event_horizon"))
                .finish()
        }
    }
}

fn import_page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::ImportSingularityConfig))
}

fn finish_page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::FinishConfigImport))
}
