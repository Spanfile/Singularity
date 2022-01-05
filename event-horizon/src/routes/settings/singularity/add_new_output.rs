use crate::{
    singularity::SingularityConfig,
    template::{
        self,
        settings::{SettingsPage, SingularitySubPage},
        Alert,
    },
    util::request_callback_error::RequestCallbackError,
};
use actix_router::PathDeserializer;
use actix_web::{
    error::UrlencodedError,
    http::{header, StatusCode},
    web, HttpRequest, HttpResponse, Responder,
};
use log::*;
use serde::{de, Deserialize};
use singularity::{
    Output, OutputType, DEFAULT_BLACKHOLE_ADDRESS_V4, DEFAULT_DEDUPLICATE, DEFAULT_METRIC_NAME, DEFAULT_OUTPUT_METRIC,
};
use std::{net::IpAddr, path::PathBuf, sync::RwLock};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
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
    fn into_output(self) -> Output {
        Output::new(
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
        .blackhole_address(self.blackhole_address)
        .deduplicate(cursed_checkbox_option(self.deduplicate))
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("{action}")
            .app_data(web::FormConfig::default().error_handler(form_error_handler))
            .route(web::get().to(add_new_output))
            .route(web::post().to(submit_form)),
    );
}

fn form_error_handler(err: UrlencodedError, req: &HttpRequest) -> actix_web::Error {
    warn!("Add new output POST failed: {}", err);
    warn!("{:?}", req);

    let req = req.clone();
    RequestCallbackError::new(StatusCode::BAD_REQUEST, move || {
        let cfg = req
            .app_data::<web::Data<RwLock<SingularityConfig>>>()
            .and_then(|cfg| cfg.read().ok())
            .expect("failed to lock read singularity config");

        let action = de::Deserialize::deserialize(PathDeserializer::new(req.match_info()))
            .expect("failed to extract output from request path");

        template::settings(
            SettingsPage::Singularity(match action {
                Action::AddNewHostsOutput => SingularitySubPage::AddNewHostsOutput,
                Action::AddNewLuaOutput => SingularitySubPage::AddNewLuaOutput,
            }),
            &cfg,
        )
        .alert(Alert::Warning(err.to_string()))
        .bad_request()
    })
    .into()
}

async fn add_new_output(action: web::Path<Action>, cfg: web::Data<RwLock<SingularityConfig>>) -> impl Responder {
    let cfg = cfg.read().expect("failed to lock read singularity config");

    template::settings(
        SettingsPage::Singularity(match action.into_inner() {
            Action::AddNewHostsOutput => SingularitySubPage::AddNewHostsOutput,
            Action::AddNewLuaOutput => SingularitySubPage::AddNewLuaOutput,
        }),
        &cfg,
    )
    .ok()
}

async fn submit_form(
    action: web::Path<Action>,
    output: web::Form<OutputForm>,
    cfg: web::Data<RwLock<SingularityConfig>>,
) -> impl Responder {
    info!("Adding output: {:#?}", output);

    let mut cfg = cfg.write().expect("failed to lock write singularity config");

    if cfg.add_output(output.into_inner().into_output()) {
        info!("Output succesfully added");

        HttpResponse::build(StatusCode::SEE_OTHER)
            .append_header((header::LOCATION, "/settings/singularity"))
            .finish()
    } else {
        warn!("Failed to add output: identical output already exists");

        template::settings(
            SettingsPage::Singularity(match action.into_inner() {
                Action::AddNewHostsOutput => SingularitySubPage::AddNewHostsOutput,
                Action::AddNewLuaOutput => SingularitySubPage::AddNewLuaOutput,
            }),
            &cfg,
        )
        .alert(Alert::Warning(
            "Failed to add output: identical output already exists.".to_string(),
        ))
        .ok()
    }
}

// see the comment at OutputForm
fn cursed_checkbox_option(opt: Option<String>) -> bool {
    !matches!(opt.as_deref(), Some("") | None)
}

fn default_blackhole_address() -> IpAddr {
    DEFAULT_BLACKHOLE_ADDRESS_V4
        .parse()
        .expect("failed to parse default blackhole address")
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
    if DEFAULT_DEDUPLICATE {
        Some(String::new())
    } else {
        None
    }
}
