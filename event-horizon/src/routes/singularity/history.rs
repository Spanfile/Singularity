use crate::{config::EvhConfig, database::DbPool, singularity::runner::history::RunnerHistory, template, util};
use actix_web::{web, Either, Responder};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/history")
            .route("", web::get().to(histories_page))
            .route("/{run_id}", web::get().to(history_page)),
    );
}

async fn histories_page(db_pool: web::Data<DbPool>) -> impl Responder {
    match db_pool.get().and_then(|mut conn| RunnerHistory::load_all(&mut conn)) {
        Ok(histories) => Either::Left(template::singularity::singularity_histories(&histories)),
        Err(e) => Either::Right(util::internal_server_error_response(format!(
            "Failed to get Singularity run histories: {}",
            e
        ))),
    }
}

async fn history_page(
    run_id: web::Path<String>,
    evh_config: web::Data<EvhConfig>,
    db_pool: web::Data<DbPool>,
) -> impl Responder {
    match db_pool
        .get()
        .and_then(|mut conn| RunnerHistory::load(&run_id, &mut conn, &evh_config))
    {
        Ok(history) => Either::Left(template::singularity::singularity_history(
            history.timestamp(),
            history.result(),
            history.events(),
        )),

        Err(e) => Either::Right(util::internal_server_error_response(format!(
            "Failed to get Singularity run histories: {}",
            e
        ))),
    }
}
