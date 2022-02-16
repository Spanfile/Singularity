use crate::{
    config::EvhConfig,
    database::{models, DbConn},
    error::EvhResult,
    logging::LogLevel,
};
use chrono::{DateTime, Local};
use diesel::prelude::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Debug)]
pub struct RunnerHistory {
    run_id: String,
    timestamp: DateTime<Local>,
    events: Vec<HistoryEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEvent {
    timestamp: f32,
    message: String,
    severity: LogLevel,
}

impl RunnerHistory {
    pub fn new(id: &str, timestamp: DateTime<Local>) -> Self {
        Self {
            run_id: id.to_string(),
            timestamp,
            events: Vec::new(),
        }
    }

    pub fn load(id: &str, conn: &mut DbConn) -> EvhResult<Self> {
        todo!()
    }

    pub fn load_all(conn: &mut DbConn) -> EvhResult<Vec<String>> {
        // return only the IDs
        todo!()
    }

    pub fn generate_filename(&self) -> String {
        // %F = &Y-%m-%d
        format!("{}-{}", self.timestamp.format("%F"), self.run_id)
    }

    pub fn save(&self, conn: &mut DbConn, evh_cfg: &EvhConfig) -> EvhResult<()> {
        use crate::database::schema::singularity_run_histories;

        let filename = self.generate_filename();
        let save_path = Path::join(&evh_cfg.history_location, &filename);

        let history = diesel::insert_into(singularity_run_histories::table)
            .values(models::NewSingularityRunHistory {
                run_id: &self.run_id,
                timestamp: &self.timestamp.to_string(),
                filename: &filename,
            })
            .get_result::<models::SingularityRunHistory>(conn)?;

        debug!("Insert Singularity run history: {:#?}", history);
        debug!("Saving history events to {}", save_path.display());

        let event_file = std::fs::File::create(&save_path)?;
        let mut writer = BufWriter::new(event_file);

        serde_json::to_writer(&mut writer, &self.events)?;
        writer.flush()?;

        Ok(())
    }

    pub fn debug(&mut self, timestamp: f32, message: String) {
        debug!("Singularity {}: {}", self.run_id, message);
        self.events.push(HistoryEvent {
            timestamp,
            message,
            severity: LogLevel::Debug,
        })
    }

    pub fn info(&mut self, timestamp: f32, message: String) {
        info!("Singularity {}: {}", self.run_id, message);
        self.events.push(HistoryEvent {
            timestamp,
            message,
            severity: LogLevel::Info,
        })
    }

    pub fn warn(&mut self, timestamp: f32, message: String) {
        warn!("Singularity {}: {}", self.run_id, message);
        self.events.push(HistoryEvent {
            timestamp,
            message,
            severity: LogLevel::Warn,
        })
    }

    pub fn error(&mut self, timestamp: f32, message: String) {
        error!("Singularity {}: {}", self.run_id, message);
        self.events.push(HistoryEvent {
            timestamp,
            message,
            severity: LogLevel::Warn,
        })
    }
}
