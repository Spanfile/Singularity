use crate::{
    singularity::singularity_runner::{CurrentlyRunningSingularity, SingularityRunner},
    template,
};
use actix_web::{http::header, web, Either, HttpResponse, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/run")
            .route(web::get().to(run_singularity_page))
            .route(web::post().to(submit_run_singularity_form)),
    );
}

async fn run_singularity_page(runner: web::Data<SingularityRunner>) -> impl Responder {
    match runner.get_currently_running() {
        None => Either::Right(
            HttpResponse::SeeOther()
                .append_header((header::LOCATION, "/singularity"))
                .finish(),
        ),
        Some(CurrentlyRunningSingularity::Running) => Either::Left(template::singularity::singularity_running()),
        Some(CurrentlyRunningSingularity::Finished) => Either::Left(template::singularity::singularity_finished()),
    }
}

async fn submit_run_singularity_form() -> impl Responder {
    // trigger the run here

    HttpResponse::SeeOther()
        .append_header((header::LOCATION, "/singularity/run"))
        .finish()
}
