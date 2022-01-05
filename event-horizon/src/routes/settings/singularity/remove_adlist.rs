use crate::{
    singularity::SingularityConfig,
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
        Alert,
    },
    util::request_callback_error::RequestCallbackError,
};
use actix_web::{
    error::UrlencodedError,
    http::{header, StatusCode},
    web, HttpRequest, HttpResponse, Responder,
};
use log::*;
use serde::Deserialize;
use std::sync::RwLock;

#[derive(Debug, Deserialize)]
struct RemoveId {
    id: u64,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/remove_adlist").service(
            web::resource("")
                .app_data(web::FormConfig::default().error_handler(form_error_handler))
                .route(web::get().to(remove_adlist))
                .route(web::post().to(submit_form)),
        ),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Remove adlist POST failed: {}", err);
    warn!("{:?}", req);

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let cfg = req
            .app_data::<web::Data<RwLock<SingularityConfig>>>()
            .and_then(|cfg| cfg.read().ok())
            .expect("failed to lock read singularity config");

        let source = web::Query::<RemoveId>::from_query(req.query_string())
            .expect("failed to extract source parameter from query");

        template::settings(
            SettingsPage::Singularity(SingularitySubPage::RemoveAdlist(source.id)),
            &cfg,
        )
        .alert(Alert::Warning(format!("Failed to remove adlist: {}", err)))
        .bad_request()
    })
    .into()
}

async fn remove_adlist(
    id: web::Query<RemoveId>,
    singularity_config: web::Data<RwLock<SingularityConfig>>,
) -> impl Responder {
    let cfg = singularity_config
        .read()
        .expect("failed to lock read singularity config");

    template::settings(SettingsPage::Singularity(SingularitySubPage::RemoveAdlist(id.id)), &cfg).ok()
}

async fn submit_form(
    id: web::Form<RemoveId>,
    singularity_config: web::Data<RwLock<SingularityConfig>>,
) -> impl Responder {
    info!("Removing adlist: {:?}", id);

    let mut cfg = singularity_config
        .write()
        .expect("failed to lock write singularity config");

    if cfg.remove_adlist(id.id) {
        info!("Adlist succesfully removed");

        HttpResponse::build(StatusCode::SEE_OTHER)
            .append_header((header::LOCATION, "/settings/singularity"))
            .finish()
    } else {
        warn!("Failed to remove adlist: no adlist with the source exists");

        template::settings(SettingsPage::Singularity(SingularitySubPage::RemoveAdlist(id.id)), &cfg)
            .alert(Alert::Warning(
                "Failed to remove adlist: no adlist with the source exists".to_string(),
            ))
            .bad_request()
    }
}
