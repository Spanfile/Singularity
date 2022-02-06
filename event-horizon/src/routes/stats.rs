use crate::{
    rec_control::{RecControl, RecControlMessage},
    template::{self, Alert},
};
use actix_web::{web, Responder};
use log::*;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/stats").route(web::get().to(stats)));
}

async fn stats(rec_control: web::Data<RecControl>) -> impl Responder {
    match rec_control.send_control_message(RecControlMessage::GetAll).await {
        Ok(resp) => {
            let stats = resp
                .lines()
                .filter_map(|line| line.split_once(char::is_whitespace))
                .collect::<Vec<(&str, &str)>>();

            template::stats(Some(&stats)).current_path("/")
        }
        Err(e) => {
            error!("Failed to get Recursor statistics: {}", e);

            template::stats(None)
                .alert(Alert::Error(format!(
                    "Failed to gather Recursor statistics due to an internal server error: {}",
                    e
                )))
                .current_path("/")
        }
    }
}
