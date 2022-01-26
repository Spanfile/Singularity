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
use serde::Deserialize;
use singularity::{
    Output, OutputType, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_DEDUPLICATE, DEFAULT_METRIC_NAME, DEFAULT_OUTPUT_METRIC,
};
use std::{net::IpAddr, path::PathBuf, sync::Arc};

#[derive(Clone, Copy)]
enum Action {
    AddNewHostsOutput,
    AddNewLuaOutput,
}

// so HTML form checkboxes are really fucking stupid. they don't emit a simple true/false boolean value for being
// checked or not, by default they emit a string "on" if they're checked, and nothing or an empty string if they're not
// checked. because of this, I can't just deserialize the form data into an Output object, oh no, I have to use an
// entirely different type that has Options in place of booleans where the values None and Some("") are false, and
// anything else is true. if it means allocating a bunch of empty strings only to discard them later, that's too fucking
// bad.
#[derive(Debug, Deserialize)]
struct OutputForm {
    #[serde(flatten)]
    ty: OutputTypeForm,
    destination: PathBuf,
    #[serde(default = "default_blackhole_address")]
    blackhole_address: IpAddr,
    #[serde(default = "default_deduplicate")]
    deduplicate: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]

enum OutputTypeForm {
    Hosts {
        #[serde(default)]
        include: Vec<PathBuf>,
    },
    PdnsLua {
        #[serde(default = "default_output_metric")]
        output_metric: Option<String>,
        #[serde(default = "default_metric_name")]
        metric_name: String,
    },
}

impl OutputForm {
    fn try_into_output(self) -> EvhResult<Output> {
        Ok(Output::builder(
            match self.ty {
                OutputTypeForm::Hosts { include } => OutputType::Hosts { include },
                OutputTypeForm::PdnsLua {
                    output_metric,
                    metric_name,
                } => OutputType::PdnsLua {
                    output_metric: cursed_checkbox_option(output_metric),
                    metric_name,
                },
            },
            self.destination,
        )
        .blackhole_ipaddr(self.blackhole_address)
        .deduplicate(cursed_checkbox_option(self.deduplicate))
        .build()?)
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/add_new_hosts_output")
            .app_data(web::FormConfig::default().error_handler(hosts_form_error_handler))
            .route(web::get().to(add_new_hosts_output_page))
            .route(web::post().to(submit_hosts_form)),
    )
    .service(
        web::resource("/add_new_lua_output")
            .app_data(web::FormConfig::default().error_handler(pdns_lua_form_error_handler))
            .route(web::get().to(add_new_lua_output_page))
            .route(web::post().to(submit_lua_form)),
    );
}

fn hosts_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(Action::AddNewHostsOutput, err, req)
}

fn pdns_lua_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(Action::AddNewLuaOutput, err, req)
}

fn form_error_handler(action: Action, err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Add new output POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        page(action)
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
            .render()
    })
    .into()
}

async fn add_new_hosts_output_page() -> impl Responder {
    add_new_output(Action::AddNewHostsOutput)
}

async fn add_new_lua_output_page() -> impl Responder {
    add_new_output(Action::AddNewLuaOutput)
}

async fn submit_hosts_form(
    output: web::Form<OutputForm>,
    cfg: web::Data<ConfigManager>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    submit_form(Action::AddNewHostsOutput, output, cfg, pool).await
}

async fn submit_lua_form(
    output: web::Form<OutputForm>,
    cfg: web::Data<ConfigManager>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    submit_form(Action::AddNewLuaOutput, output, cfg, pool).await
}

fn add_new_output(action: Action) -> impl Responder {
    page(action)
}

async fn submit_form(
    action: Action,
    output: web::Form<OutputForm>,
    cfg: web::Data<ConfigManager>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    info!("Adding output: {:#?}", output);

    match add_output(output.into_inner(), cfg.get_active_config(), pool.into_inner()).await {
        Ok(_) => {
            info!("Output succesfully added");
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
            ))
            | EvhError::Singularity(_) => {
                warn!("Failed to add output: an output with the same destination already exists");
                warn!("{}", e);

                Either::Left(
                    page(action)
                        .alert(Alert::Warning(
                            "An output with the same destination already exists".to_string(),
                        ))
                        .bad_request(),
                )
            }

            e => {
                error!("Failed to add output: {}", e);

                Either::Left(
                    page(action)
                        .alert(Alert::Error(format!(
                            "Failed to add output due to an internal server error: {}",
                            e
                        )))
                        .internal_server_error(),
                )
            }
        },
    }
}

async fn add_output(form: OutputForm, cfg: Arc<SingularityConfig>, pool: Arc<DbPool>) -> EvhResult<()> {
    web::block(move || {
        let mut conn = pool.get().map_err(EvhError::DatabaseConnectionAcquireFailed)?;

        form.try_into_output()
            .and_then(|output| cfg.add_output(&mut conn, &output))
    })
    .await
    .unwrap()?;

    Ok(())
}

fn page<'a>(action: Action) -> ResponseBuilder<'a> {
    template::settings(SettingsPage::Singularity(match action {
        Action::AddNewHostsOutput => SingularitySubPage::AddNewHostsOutput,
        Action::AddNewLuaOutput => SingularitySubPage::AddNewLuaOutput,
    }))
}

// see the comment at OutputForm
fn cursed_checkbox_option(opt: Option<String>) -> bool {
    !matches!(opt.as_deref(), Some("") | None)
}

fn default_blackhole_address() -> IpAddr {
    DEFAULT_BLACKHOLE_ADDRESS_V4
}

fn default_output_metric() -> Option<String> {
    if DEFAULT_OUTPUT_METRIC {
        Some(String::new())
    } else {
        None
    }
}

fn default_metric_name() -> String {
    String::from(DEFAULT_METRIC_NAME)
}

fn default_deduplicate() -> Option<String> {
    if DEFAULT_DEDUPLICATE { Some(String::new()) } else { None }
}
