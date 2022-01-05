use crate::{
    singularity::SingularityConfig,
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
        Alert,
    },
    util::request_callback_error::RequestCallbackError,
};
use actix_router::PathDeserializer;
use actix_web::{
    error::UrlencodedError,
    http::{header, StatusCode},
    web, HttpRequest, HttpResponse, Responder,
};
use log::*;
use serde::{de, Deserialize};
use singularity::Output;
use std::sync::RwLock;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    AddNewHostsOutput,
    AddNewLuaOutput,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("{action}")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(add_new_output))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Add new output POST failed: {}", err);
    warn!("{:?}", req);

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let cfg = req
            .app_data::<web::Data<RwLock<SingularityConfig>>>()
            .and_then(|cfg| cfg.read().ok())
            .expect("failed to lock read singularity config");

        let action = de::Deserialize::deserialize(PathDeserializer::new(req.match_info()))
            .expect("failed to extract output from request path");

        template::settings(
            SettingsPage::Singularity(match action {
                Action::AddNewHostsOutput => SingularitySubPage::AddNewHostsOutput,
                Action::AddNewLuaOutput => SingularitySubPage::AddNewLuaOutput,
            }),
            &cfg,
        )
        .alert(Alert::Warning(err.to_string()))
        .bad_request()
    })
    .into()
}

async fn add_new_output(action: web::Path<Action>, cfg: web::Data<RwLock<SingularityConfig>>) -> impl Responder {
    let cfg = cfg.read().expect("failed to lock read singularity config");

    template::settings(
        SettingsPage::Singularity(match action.into_inner() {
            Action::AddNewHostsOutput => SingularitySubPage::AddNewHostsOutput,
            Action::AddNewLuaOutput => SingularitySubPage::AddNewLuaOutput,
        }),
        &cfg,
    )
    .ok()
}

async fn submit_form(
    action: web::Path<Action>,
    output: web::Form<Output>,
    cfg: web::Data<RwLock<SingularityConfig>>,
) -> impl Responder {
    info!("Adding output: {:?}", output);

    let mut cfg = cfg.write().expect("failed to lock write singularity config");

    if cfg.add_output(output.into_inner()) {
        info!("Output succesfully added");

        HttpResponse::build(StatusCode::SEE_OTHER)
            .append_header((header::LOCATION, "/settings/singularity"))
            .finish()
    } else {
        warn!("Failed to add output: identical output already exists");

        template::settings(
            SettingsPage::Singularity(match action.into_inner() {
                Action::AddNewHostsOutput => SingularitySubPage::AddNewHostsOutput,
                Action::AddNewLuaOutput => SingularitySubPage::AddNewLuaOutput,
            }),
            &cfg,
        )
        .alert(Alert::Warning(
            "Failed to add output: identical output already exists.".to_string(),
        ))
        .ok()
    }
}
