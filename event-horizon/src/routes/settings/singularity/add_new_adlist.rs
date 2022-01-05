use crate::{
    singularity::SingularityConfig,
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
        Alert,
    },
    util::request_wrapped_error::{RequestWrappedError, WrappedError},
};
use actix_web::{
    error::UrlencodedError,
    http::{header, StatusCode},
    web, HttpRequest, HttpResponse, Responder,
};
use log::*;
use singularity::Adlist;
use std::sync::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("")]
struct FormError {
    err: UrlencodedError,
}

impl WrappedError for FormError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn error_response(&self, req: &HttpRequest) -> HttpResponse {
        let cfg = req
            .app_data::<RwLock<SingularityConfig>>()
            .expect("failed to get singularity config rwlock")
            .read()
            .expect("failed to lock read singularity config");

        template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist), &cfg)
            .alert(Alert::Warning(self.to_string()))
            .bad_request()
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/add_new_adlist").service(
            web::resource("")
                .app_data(web::FormConfig::default().error_handler(form_error_handler))
                .route(web::get().to(add_new_adlist))
                .route(web::post().to(submit_form)),
        ),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Add new adlist POST failed: {}", err);
    warn!("{:?}", req);

    RequestWrappedError::new(FormError { err }, req).into()
}

async fn add_new_adlist(singularity_config: web::Data<RwLock<SingularityConfig>>) -> impl Responder {
    let cfg = singularity_config
        .read()
        .expect("failed to lock read singularity config");

    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist), &cfg).ok()
}

async fn submit_form(
    adlist: web::Form<Adlist>,
    singularity_config: web::Data<RwLock<SingularityConfig>>,
) -> impl Responder {
    info!("Adding new adlist: {:?}", adlist);

    let mut cfg = singularity_config
        .write()
        .expect("failed to lock write singularity config");

    if cfg.add_adlist(adlist.into_inner()) {
        info!("Adlist succesfully added");

        HttpResponse::build(StatusCode::SEE_OTHER)
            .append_header((header::LOCATION, "/settings/singularity"))
            .finish()
    } else {
        warn!("Failed to add adlist: adlist with the same source URL already exists");

        template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist), &cfg)
            .alert(Alert::Warning(
                "Failed to add new adlist: adlist with the same source URL already exists.".to_string(),
            ))
            .ok()
    }
}
