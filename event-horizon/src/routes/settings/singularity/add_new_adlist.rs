use crate::{
    database::DbPool,
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
use singularity::Adlist;
use std::sync::RwLock;

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

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let cfg = req
            .app_data::<web::Data<RwLock<SingularityConfig>>>()
            .and_then(|cfg| cfg.read().ok())
            .expect("failed to lock read singularity config");

        template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist))
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
    })
    .into()
}

async fn add_new_adlist(singularity_config: web::Data<RwLock<SingularityConfig>>) -> impl Responder {
    let cfg = singularity_config
        .read()
        .expect("failed to lock read singularity config");

    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist)).ok()
}

async fn submit_form(
    adlist: web::Form<Adlist>,
    singularity_config: web::Data<RwLock<SingularityConfig>>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    info!("Adding new adlist: {:?}", adlist);

    let cfg = singularity_config
        .write()
        .expect("failed to lock write singularity config");

    let mut conn = pool.get().expect("failed to get DB connection");
    match cfg.add_adlist(&mut conn, adlist.into_inner()) {
        Ok(_) => {
            info!("Adlist succesfully added");

            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/singularity"))
                .finish()
        }
        Err(e) => {
            warn!("Failed to add adlist: {}", e);

            template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist))
                .alert(Alert::Warning(format!("Failed to add new adlist: {}", e)))
                .ok()
        }
    }
}
