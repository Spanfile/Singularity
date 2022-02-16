use crate::{
    config::EvhConfig,
    database::{models, DbConn},
    error::{EvhError, EvhResult},
    logging::LogLevel,
};
use chrono::{DateTime, Local};
use diesel::prelude::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
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

    pub fn load(run_id: &str, conn: &mut DbConn, evh_cfg: &EvhConfig) -> EvhResult<Self> {
        use crate::database::schema::singularity_run_histories;

        let hist = singularity_run_histories::table
            .filter(singularity_run_histories::run_id.eq(run_id))
            .first::<models::SingularityRunHistory>(conn)
            .optional()?
            .ok_or_else(|| EvhError::NoSuchHistory(run_id.to_string()))?;

        debug!("Singularity history: {:?}", hist);

        let timestamp: DateTime<Local> = hist.timestamp.parse()?;
        let events_filename = generate_filename(timestamp, run_id);
        let events_path = Path::join(&evh_cfg.history_location, &events_filename);

        let events_file = File::open(&events_path)?;
        let mut reader = BufReader::new(events_file);
        let events: Vec<HistoryEvent> = serde_json::from_reader(&mut reader)?;

        debug!(
            "Singularity history {} @ {}: {} events",
            run_id,
            timestamp,
            events.len()
        );

        Ok(Self {
            run_id: run_id.to_string(),
            timestamp,
            events,
        })
    }

    pub fn load_all(conn: &mut DbConn) -> EvhResult<Vec<(String, DateTime<Local>)>> {
        use crate::database::schema::singularity_run_histories;

        let histories = singularity_run_histories::table
            .load::<models::SingularityRunHistory>(conn)?
            .into_iter()
            .map(|hist| Ok((hist.run_id, hist.timestamp.parse()?)))
            .collect::<EvhResult<Vec<_>>>()?;

        debug!("Singularity run histories: {}", histories.len());
        Ok(histories)
    }

    pub fn timestamp(&self) -> DateTime<Local> {
        self.timestamp
    }

    pub fn events(&self) -> &[HistoryEvent] {
        &self.events
    }

    pub fn save(&self, conn: &mut DbConn, evh_cfg: &EvhConfig) -> EvhResult<()> {
        use crate::database::schema::singularity_run_histories;

        let filename = generate_filename(self.timestamp, &self.run_id);
        let save_path = Path::join(&evh_cfg.history_location, &filename);

        let history = diesel::insert_into(singularity_run_histories::table)
            .values(models::NewSingularityRunHistory {
                run_id: &self.run_id,
                timestamp: &self.timestamp.to_string(),
            })
            .get_result::<models::SingularityRunHistory>(conn)?;

        debug!("Insert Singularity run history: {:#?}", history);
        debug!("Saving history events to {}", save_path.display());

        let event_file = File::create(&save_path)?;
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

impl HistoryEvent {
    pub fn timestamp(&self) -> f32 {
        self.timestamp
    }

    pub fn severity(&self) -> LogLevel {
        self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

fn generate_filename(timestamp: DateTime<Local>, run_id: &str) -> String {
    // %F = &Y-%m-%d
    format!("{}-{}", timestamp.format("%F"), run_id)
}
