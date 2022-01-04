use crate::{
    singularity::SingularityConfig,
    template::{self, settings::SettingsPage, Alert, ResponseBuilder},
    util::request_wrapped_error::{RequestWrappedError, WrappedError},
};
use ::singularity::Adlist;
use actix_web::{error::UrlencodedError, http::StatusCode, web, HttpRequest, HttpResponse, Responder};
use log::*;
use serde::Deserialize;
use singularity::Output;
use std::sync::RwLock;
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(tag = "submitted_form", rename_all = "snake_case")]
enum SubmittedForm {
    General,
    AddAdlist(Adlist),
    // the enum field has to be a map because of the tagged representation
    RemoveAdlist { source: String },
    AddOutput(Output),
    RemoveOutput(Output),
}

#[derive(Debug, Error)]
enum SettingsError {
    #[error(transparent)]
    InvalidForm(#[from] UrlencodedError),
}

impl WrappedError for SettingsError {
    fn status_code(&self) -> StatusCode {
        match self {
            SettingsError::InvalidForm(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self, req: &HttpRequest) -> HttpResponse {
        let cfg = req
            .app_data::<RwLock<SingularityConfig>>()
            .expect("failed to get singularity config rwlock")
            .read()
            .expect("failed to lock read singularity config");

        page_with_alert(Alert::Warning(self.to_string()), &cfg).bad_request()
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/singularity").service(
            web::resource("")
                .app_data(web::FormConfig::default().error_handler(post_error_handler))
                .route(web::get().to(singularity))
                .route(web::post().to(post_form)),
        ),
    );
}

fn page(singularity_config: &SingularityConfig) -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity, singularity_config)
}

fn page_with_alert(alert: Alert, singularity_config: &SingularityConfig) -> ResponseBuilder<'static> {
    page(singularity_config).alert(alert)
}

fn post_error_handler(e: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Singularity POST failed: {}", e);
    warn!("{:?}", req);

    RequestWrappedError::new(SettingsError::InvalidForm(e), req).into()
}

async fn singularity(singularity_config: web::Data<RwLock<SingularityConfig>>) -> impl Responder {
    let cfg = singularity_config
        .read()
        .expect("failed to lock read singularity config");

    page(&cfg).ok()
}

async fn post_form(
    form: web::Form<SubmittedForm>,
    singularity_config: web::Data<RwLock<SingularityConfig>>,
) -> impl Responder {
    debug!("Singularity POST form: {:?}", form);

    let mut cfg = singularity_config
        .write()
        .expect("failed to lock write singularity config");

    match form.into_inner() {
        SubmittedForm::General => todo!(),

        SubmittedForm::AddAdlist(adlist) => {
            info!("Adding new adlist: {:?}", adlist);

            if cfg.add_adlist(adlist) {
                info!("Adlist succesfully added");
                page_with_alert(Alert::Success("New adlist succesfully added!".to_string()), &cfg).ok()
            } else {
                warn!("Failed to add adlist: adlist with the same source URL already exists");
                page_with_alert(
                    Alert::Warning(
                        "Failed to add new adlist: adlist with the same source URL already exists.".to_string(),
                    ),
                    &cfg,
                )
                .ok()
            }
        }
        SubmittedForm::RemoveAdlist { source } => {
            info!("Removing adlist: {}", source);

            if cfg.remove_adlist(&source) {
                info!("Adlist succesfully removed");
                page_with_alert(Alert::Success("Adlist succesfully removed.".to_string()), &cfg).ok()
            } else {
                warn!("Failed to remove adlist: no adlist with such source URL present");
                page_with_alert(
                    Alert::Warning("Failed to remove adlist: no adlist with such source URL present".to_string()),
                    &cfg,
                )
                .ok()
            }
        }

        SubmittedForm::AddOutput(_) => todo!(),
        SubmittedForm::RemoveOutput(_) => todo!(),
    }
}
