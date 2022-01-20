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
    web, Either, HttpRequest, HttpResponse, Responder,
};
use log::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WhitelistedDomain {
    domain: String,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/add_whitelisted_domain")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(add_whitelisted_domain))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Add new whitelisted domain POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        page().alert(Alert::Warning(err.to_string())).bad_request().render()
    })
    .into()
}

async fn add_whitelisted_domain() -> impl Responder {
    page()
}

async fn submit_form(
    domain: web::Form<WhitelistedDomain>,
    cfg: web::Data<SingularityConfig>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    info!("Adding new whitelisted domain: {:?}", domain);

    match add_domain(domain.into_inner().domain, &cfg, &pool) {
        Ok(_) => {
            info!("Whitelisted domain succesfully added");

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
                warn!("Failed to add whitelisted domain: duplicate domain");
                warn!("{}", e);

                Either::Left(
                    page()
                        .alert(Alert::Warning("Duplicate whitelisted domain".to_string()))
                        .bad_request(),
                )
            }
            _ => {
                error!("Failed to add whitelisted domain: {}", e);

                Either::Left(
                    page()
                        .alert(Alert::Warning(format!(
                            "Failed to add whitelisted domain due to internal server error: {}",
                            e
                        )))
                        .internal_server_error(),
                )
            }
        },
    }
}

fn add_domain(domain: String, cfg: &SingularityConfig, pool: &DbPool) -> EvhResult<()> {
    let mut conn = pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;
    cfg.add_whitelisted_domain(&mut conn, &domain)?;
    Ok(())
}

fn page() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewWhitelistedDomain))
}
