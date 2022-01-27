use crate::{
    database::DbPool,
    error::{EvhError, EvhResult},
    singularity::{ConfigManager, SingularityConfig},
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
    web, Either, HttpRequest, HttpResponse, Responder,
};
use log::*;
use singularity::Adlist;
use std::sync::Arc;

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
        page().alert(Alert::Warning(err.to_string())).bad_request().render()
    })
    .into()
}

async fn add_new_adlist() -> impl Responder {
    page()
}

async fn submit_form(
    adlist: web::Form<Adlist>,
    cfg_mg: web::Data<ConfigManager>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    info!("Adding new adlist: {:?}", adlist);

    match add_adlist(adlist.into_inner(), cfg_mg.get_active_config(), pool.into_inner()).await {
        Ok(_) => {
            info!("Adlist succesfully added");
            Either::Right(
                HttpResponse::build(StatusCode::SEE_OTHER)
                    .append_header((header::LOCATION, "/settings/singularity"))
                    .finish(),
            )
        }
        Err(e) => match e {
            EvhError::Database(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => {
                warn!("Failed to add adlist: an adlist with the same source already exists");
                warn!("{}", e);

                Either::Left(
                    page()
                        .alert(Alert::Warning(
                            "An adlist with the same source already exists".to_string(),
                        ))
                        .bad_request(),
                )
            }

            _ => {
                error!("Failed to add adlist: {}", e);

                Either::Left(
                    page()
                        .alert(Alert::Error(format!(
                            "Failed to add new adlist due to an internal server error: {}",
                            e
                        )))
                        .internal_server_error(),
                )
            }
        },
    }
}

async fn add_adlist(adlist: Adlist, cfg: SingularityConfig, pool: Arc<DbPool>) -> EvhResult<()> {
    web::block(move || {
        let mut conn = pool.get()?;
        cfg.add_adlist(&mut conn, &adlist)
    })
    .await
    .unwrap()?;

    Ok(())
}

fn page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist))
}
