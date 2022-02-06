use crate::{
    rec_control::{RecControl, RecControlMessage},
    template::{self, Alert},
};
use actix_web::{web, Responder};
use log::*;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/statistics")
            .route("", web::route().to(stats))
            .route("/recursor", web::route().to(raw_recursor)),
    );
}

async fn stats() -> impl Responder {
    template::stats()
}

async fn raw_recursor(rec_control: web::Data<RecControl>) -> impl Responder {
    match rec_control.send_control_message(RecControlMessage::GetAll).await {
        Ok(resp) => {
            let recursor_stats = resp
                .lines()
                .filter_map(|line| line.split_once(char::is_whitespace))
                .collect::<Vec<_>>();

            template::stats::raw_recursor(Some(&recursor_stats))
        }
        Err(e) => {
            error!("Failed to get Recursor statistics: {}", e);

            template::stats::raw_recursor(None).alert(Alert::Error(format!(
                "Failed to gather Recursor statistics due to an internal server error: {}",
                e
            )))
        }
    }
}
