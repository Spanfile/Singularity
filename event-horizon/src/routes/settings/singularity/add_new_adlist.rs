use crate::{
    database::DbPool,
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
use singularity::Adlist;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/add_new_adlist")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(add_new_adlist))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Add new adlist POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        page().alert(Alert::Warning(err.to_string())).bad_request()
    })
    .into()
}

async fn add_new_adlist() -> impl Responder {
    page().ok()
}

async fn submit_form(
    adlist: web::Form<Adlist>,
    cfg: web::Data<SingularityConfig>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    info!("Adding new adlist: {:?}", adlist);

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

            page()
                .alert(Alert::Warning(format!("Failed to add new adlist: {}", e)))
                .ok()
        }
    }
}

fn page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist))
}
