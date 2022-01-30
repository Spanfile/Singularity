use crate::{
    database::{DbConn, DbId, DbPool},
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
    Adlist, Output, OutputType, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_DEDUPLICATE, DEFAULT_METRIC_NAME,
    DEFAULT_OUTPUT_METRIC,
};
use std::{net::IpAddr, path::PathBuf, sync::Arc};

#[derive(Debug, Deserialize)]
struct WhitelistForm {
    domain: String,
}

// so HTML form checkboxes are really fucking stupid. they don't emit a simple true/false boolean value for being
// checked or not, but instead they emit a string "on" if they're checked, and nothing or an empty string if they're not
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
        web::resource("/add_new_adlist")
            .app_data(web::FormConfig::default().error_handler(adlist_form_error_handler))
            .route(web::get().to(add_new_adlist_page))
            .route(web::post().to(submit_adlist_form)),
    )
    .service(
        web::resource("/add_new_hosts_output")
            .app_data(web::FormConfig::default().error_handler(hosts_output_form_error_handler))
            .route(web::get().to(add_new_hosts_output_page))
            .route(web::post().to(submit_hosts_output_form)),
    )
    .service(
        web::resource("/add_new_lua_output")
            .app_data(web::FormConfig::default().error_handler(lua_output_form_error_handler))
            .route(web::get().to(add_new_lua_output_page))
            .route(web::post().to(submit_lua_output_form)),
    )
    .service(
        web::resource("/add_whitelisted_domain")
            .app_data(web::FormConfig::default().error_handler(whitelist_form_error_handler))
            .route(web::get().to(add_new_whitelist_page))
            .route(web::post().to(submit_whitelist_form)),
    );
}

fn adlist_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(err, req, adlist_template)
}

fn hosts_output_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(err, req, hosts_template)
}

fn lua_output_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(err, req, lua_template)
}

fn whitelist_form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    form_error_handler(err, req, whitelist_template)
}

fn form_error_handler<P>(err: UrlencodedError, req: &HttpRequest, template_fn: P) -> actix_web::Error
where
    P: Fn() -> ResponseBuilder<'static> + 'static,
{
    warn!("Add new item POST failed: {}", err);
    warn!("{:?}", req);

    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        Ok((template_fn)()
            .alert(Alert::Warning(err.to_string()))
            .bad_request()
            .render())
    })
    .into()
}

async fn add_new_adlist_page() -> impl Responder {
    adlist_template()
}

async fn add_new_hosts_output_page() -> impl Responder {
    hosts_template()
}

async fn add_new_lua_output_page() -> impl Responder {
    lua_template()
}

async fn add_new_whitelist_page() -> impl Responder {
    whitelist_template()
}

async fn submit_adlist_form(
    adlist_form: web::Form<Adlist>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    submit_form(
        adlist_form.into_inner(),
        cfg_mg.get_active_config(),
        db_pool.into_inner(),
        |cfg, conn, adlist| cfg.add_adlist(conn, &adlist),
        adlist_template,
    )
    .await
}

async fn submit_hosts_output_form(
    output_form: web::Form<OutputForm>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    submit_form(
        output_form.into_inner(),
        cfg_mg.get_active_config(),
        db_pool.into_inner(),
        |cfg, conn, form| form.try_into_output().and_then(|output| cfg.add_output(conn, &output)),
        hosts_template,
    )
    .await
}

async fn submit_lua_output_form(
    output_form: web::Form<OutputForm>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    submit_form(
        output_form.into_inner(),
        cfg_mg.get_active_config(),
        db_pool.into_inner(),
        |cfg, conn, form| form.try_into_output().and_then(|output| cfg.add_output(conn, &output)),
        lua_template,
    )
    .await
}

async fn submit_whitelist_form(
    whitelist_form: web::Form<WhitelistForm>,
    cfg_mg: web::Data<ConfigManager>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    submit_form(
        whitelist_form.into_inner().domain,
        cfg_mg.get_active_config(),
        db_pool.into_inner(),
        |cfg, conn, domain| cfg.add_whitelisted_domain(conn, &domain),
        whitelist_template,
    )
    .await
}

async fn submit_form<T, F, P>(
    item: T,
    cfg: SingularityConfig,
    db_pool: Arc<DbPool>,
    add_fn: F,
    template_fn: P,
) -> impl Responder
where
    T: std::fmt::Debug + Send + 'static,
    F: Fn(&SingularityConfig, &mut DbConn, T) -> EvhResult<DbId> + Send + 'static,
    P: Fn() -> ResponseBuilder<'static>,
{
    info!("Adding new item to config ID {}: {:?}", cfg.id(), item);

    match web::block(move || {
        let mut conn = db_pool.get()?;
        (add_fn)(&cfg, &mut conn, item)
    })
    .await
    .expect("failed to spawn task in blocking thread pool")
    {
        Ok(id) => {
            info!("Item succesfully added: ID {}", id);

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
                warn!("Failed to add item: duplicate item");
                warn!("{}", e);

                Either::Left(
                    (template_fn)()
                        .alert(Alert::Warning("An identical item already exists".to_string()))
                        .bad_request(),
                )
            }

            _ => {
                error!("Failed to add item: {}", e);

                Either::Left(
                    (template_fn)()
                        .alert(Alert::Error(format!(
                            "Failed to add new item due to an internal server error: {}",
                            e
                        )))
                        .internal_server_error(),
                )
            }
        },
    }
}

fn adlist_template() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewAdlist))
}

fn hosts_template() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewHostsOutput))
}

fn lua_template() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewLuaOutput))
}

fn whitelist_template() -> ResponseBuilder<'static> {
    template::settings(SettingsPage::Singularity(SingularitySubPage::AddNewWhitelistedDomain))
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
