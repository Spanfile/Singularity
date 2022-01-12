use crate::{
    database::DbPool,
    error::{EvhError, EvhResult},
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

    match add_adlist(adlist.into_inner(), &cfg, &pool) {
        Ok(_) => {
            info!("Adlist succesfully added");
            HttpResponse::build(StatusCode::SEE_OTHER)
                .append_header((header::LOCATION, "/settings/singularity"))
                .finish()
        }
        Err(e) => match e {
            EvhError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => {
                warn!("Failed to add adlist: an adlist with the same source already exists");
                warn!("{}", e);

                page()
                    .alert(Alert::Warning(
                        "An adlist with the same source already exists".to_string(),
                    ))
                    .bad_request()
            }

            _ => {
                error!("Failed to add adlist: {}", e);

                page()
                    .alert(Alert::Error(format!(
                        "Failed to add new adlist due to an internal server error: {}",
                        e
                    )))
                    .internal_server_error()
            }
        },
    }
}

fn add_adlist(adlist: Adlist, cfg: &SingularityConfig, pool: &DbPool) -> EvhResult<()> {
    let mut conn = pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
    cfg.add_adlist(&mut conn, adlist)?;
    Ok(())
}

fn page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist))
}
