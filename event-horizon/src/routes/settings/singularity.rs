use crate::template::{self, settings::SettingsPage, Alert};
use ::singularity::Adlist;
use actix_web::{error::UrlencodedError, http::StatusCode, web, HttpRequest, HttpResponse, Responder};
use log::*;
use serde::Deserialize;
use singularity::OutputConfig;
use thiserror::Error;

#[derive(Debug, Error)]
enum SettingsError {
    #[error(transparent)]
    InvalidForm(#[from] UrlencodedError),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "saved_form", rename_all = "snake_case")]
enum SettingsForm {
    General,
    Adlist(Adlist),
    Output(OutputConfig),
}

impl actix_web::error::ResponseError for SettingsError {
    fn status_code(&self) -> StatusCode {
        match self {
            SettingsError::InvalidForm(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        template::settings(SettingsPage::Singularity)
            .alert(Alert::Warning(self.to_string()))
            .bad_request()
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

async fn singularity() -> impl Responder {
    template::settings(SettingsPage::Singularity).ok()
}

async fn post_form(form: web::Form<SettingsForm>) -> impl Responder {
    debug!("Singularity POST form: {:?}", form);

    match form.into_inner() {
        SettingsForm::General => todo!(),
        SettingsForm::Adlist(_) => template::settings(SettingsPage::Singularity)
            .alert(Alert::Success("New adlist succesfully added!".to_string()))
            .ok(),
        SettingsForm::Output(_) => todo!(),
    }
}

fn post_error_handler(e: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Singularity POST failed: {}", e);
    warn!("{:?}", req);

    SettingsError::InvalidForm(e).into()
}
