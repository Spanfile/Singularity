use crate::{
    database::{DbConn, DbId, DbPool},
    singularity::SingularityConfig,
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
        Alert, ResponseBuilder,
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
struct DeleteId {
    id: DbId,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/delete_output")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(delete_output))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Delete output POST failed: {}", err);
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

        let source = web::Query::<DeleteId>::from_query(req.query_string())
            .expect("failed to extract source parameter from query");

        page(source.id, &mut conn, &cfg)
            .alert(Alert::Warning(format!("Failed to delete output: {}", err)))
            .bad_request()
    })
    .into()
}

async fn delete_output(
    id: web::Query<DeleteId>,
    cfg: web::Data<RwLock<SingularityConfig>>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let cfg = cfg.read().expect("failed to lock read singularity config");
    let mut conn = pool.get().expect("failed to get DB connection");

    page(id.id, &mut conn, &cfg).ok()
}

async fn submit_form(
    id: web::Form<DeleteId>,
    singularity_config: web::Data<RwLock<SingularityConfig>>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    info!("Deleting output: {:?}", id);

    let cfg = singularity_config
        .write()
        .expect("failed to lock write singularity config");
    let mut conn = pool.get().expect("failed to get DB connection");

    match cfg.delete_output(&mut conn, id.id) {
        Ok(_) => {
            info!("Output succesfully deleted");

            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/singularity"))
                .finish()
        }
        Err(e) => {
            warn!("Failed to delete output: {}", e);

            page(id.id, &mut conn, &cfg)
                .alert(Alert::Warning(format!("Failed to delete output: {}", e)))
                .bad_request()
        }
    }
}

fn page<'a>(id: DbId, conn: &'a mut DbConn, cfg: &'a SingularityConfig) -> ResponseBuilder<'a> {
    let output = cfg.get_output(conn, id).expect("failed to get output");

    template::settings(SettingsPage::Singularity(SingularitySubPage::DeleteOutput(id, &output)))
}
