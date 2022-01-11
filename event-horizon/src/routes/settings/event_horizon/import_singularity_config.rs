use crate::{
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

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/import_singularity_config")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(import_singularity_config))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Import Singularity config POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        page().alert(Alert::Warning(err.to_string())).bad_request()
    })
    .into()
}

async fn import_singularity_config() -> impl Responder {
    page().ok()
}

// TODO: this invokes the form error handler if the left side (the form) fails. make it not do that
async fn submit_form(payload: Either<web::Form<TextImport>, Multipart>) -> impl Responder {
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
                            return page()
                                .alert(Alert::Warning(
                                    "Reading file failed: file is not encoded in UTF-8".to_string(),
                                ))
                                .bad_request();
                        }
                    }
                }
                Err(e) => {
                    return page()
                        .alert(Alert::Error(format!("Reading file failed: {:?}", e)))
                        .internal_server_error();
                }
            }
        }
    };

    debug!("Received config:\n{}", content);

    HttpResponse::build(StatusCode::SEE_OTHER)
        .append_header((header::LOCATION, "/settings/event_horizon"))
        .finish()
}

fn page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::EventHorizon(EventHorizonSubPage::ImportSingularityConfig))
}
