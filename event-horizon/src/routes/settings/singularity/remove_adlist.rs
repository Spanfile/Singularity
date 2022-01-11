use crate::{
    database::{DbId, DbPool},
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
    id: DbId,
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
        let mut conn = req
            .app_data::<web::Data<DbPool>>()
            .and_then(|pool| pool.get().ok())
            .expect("failed to get DB connection");

        let source = web::Query::<RemoveId>::from_query(req.query_string())
            .expect("failed to extract source parameter from query");
        let adlist = cfg.get_adlist(&mut conn, source.id).expect("failed to get adlist");

        template::settings(SettingsPage::Singularity(SingularitySubPage::RemoveAdlist(
            source.id, &adlist,
        )))
        .alert(Alert::Warning(format!("Failed to remove adlist: {}", err)))
        .bad_request()
    })
    .into()
}

async fn remove_adlist(
    id: web::Query<RemoveId>,
    cfg: web::Data<RwLock<SingularityConfig>>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let cfg = cfg.read().expect("failed to lock read singularity config");
    let mut conn = pool.get().expect("failed to get DB connection");
    let adlist = cfg.get_adlist(&mut conn, id.id).expect("failed to get adlist");

    template::settings(SettingsPage::Singularity(SingularitySubPage::RemoveAdlist(
        id.id, &adlist,
    )))
    .ok()
}

async fn submit_form(
    id: web::Form<RemoveId>,
    singularity_config: web::Data<RwLock<SingularityConfig>>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    info!("Removing adlist: {:?}", id);

    let cfg = singularity_config
        .write()
        .expect("failed to lock write singularity config");
    let mut conn = pool.get().expect("failed to get DB connection");

    match cfg.remove_adlist(&mut conn, id.id) {
        Ok(_) => {
            info!("Adlist succesfully removed");

            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/singularity"))
                .finish()
        }
        Err(e) => {
            warn!("Failed to remove adlist: {}", e);

            let adlist = cfg.get_adlist(&mut conn, id.id).expect("failed to get adlist");

            template::settings(SettingsPage::Singularity(SingularitySubPage::RemoveAdlist(
                id.id, &adlist,
            )))
            .alert(Alert::Warning(format!("Failed to remove adlist: {}", e)))
            .bad_request()
        }
    }
}
