use crate::{
    config::EvhConfig,
    database::{
        models::{self, SingularityRunHistoryResult},
        DbConn,
    },
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
    time::Instant,
};

#[derive(Debug)]
pub struct RunnerHistory {
    run_id: String,
    start_time: Option<Instant>,
    timestamp: DateTime<Local>,
    result: Option<SingularityRunHistoryResult>,
    events: Vec<HistoryEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEvent {
    timestamp: f32,
    message: String,
    severity: LogLevel,
}

impl PartialEq for HistoryEvent {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl PartialOrd for HistoryEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

impl RunnerHistory {
    pub fn new(id: &str, timestamp: DateTime<Local>) -> Self {
        Self {
            run_id: id.to_string(),
            start_time: Some(Instant::now()),
            timestamp,
            result: None,
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
        let mut events: Vec<HistoryEvent> = serde_json::from_reader(&mut reader)?;

        // the events are likely already sorted by timestamp because they're saved in insertion order, but sort them
        // just in case and hope no timestamp is NaN. an unstable sort is used because there very likely aren't two
        // events with the exact same timestamps
        events.sort_unstable_by(|e1, e2| e1.partial_cmp(e2).expect("timestamp in history event is NaN"));

        debug!(
            "Singularity history {} @ {}: {} events",
            run_id,
            timestamp,
            events.len()
        );

        Ok(Self {
            run_id: run_id.to_string(),
            start_time: None,
            timestamp,
            result: Some(hist.result),
            events,
        })
    }

    pub fn load_all(conn: &mut DbConn) -> EvhResult<Vec<(String, SingularityRunHistoryResult, DateTime<Local>)>> {
        use crate::database::schema::singularity_run_histories;

        let histories = singularity_run_histories::table
            // the table is likely ordered with the oldest history first, so order them by descending timestamps to
            // reverse the order and keep them sorted
            .order(singularity_run_histories::timestamp.desc())
            .load::<models::SingularityRunHistory>(conn)?
            .into_iter()
            .map(|hist| Ok((hist.run_id, hist.result, hist.timestamp.parse()?)))
            .collect::<EvhResult<Vec<_>>>()?;

        debug!("Singularity run histories: {}", histories.len());
        Ok(histories)
    }

    pub fn timestamp(&self) -> DateTime<Local> {
        self.timestamp
    }

    pub fn result(&self) -> SingularityRunHistoryResult {
        self.result.expect("run result not set in runner history")
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
                result: self.result.ok_or(EvhError::RunHistoryResultNotSet)?,
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

    pub fn set_result(&mut self, result: SingularityRunHistoryResult) {
        self.result = Some(result);
    }

    pub fn debug(&mut self, message: String) {
        debug!("Singularity {}: {}", self.run_id, message);
        self.push(message, LogLevel::Debug);
    }

    pub fn info(&mut self, message: String) {
        info!("Singularity {}: {}", self.run_id, message);
        self.push(message, LogLevel::Info);
    }

    pub fn warn(&mut self, message: String) {
        warn!("Singularity {}: {}", self.run_id, message);
        self.push(message, LogLevel::Warn);
    }

    pub fn error(&mut self, message: String) {
        error!("Singularity {}: {}", self.run_id, message);
        self.push(message, LogLevel::Error);
    }

    fn push(&mut self, message: String, severity: LogLevel) {
        self.events.push(HistoryEvent {
            timestamp: self
                .start_time
                .expect("start time not set in runner history")
                .elapsed()
                .as_secs_f32(),
            message,
            severity,
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
    format!("{}-{}", timestamp.format("%F-%H-%M-%S"), run_id)
}
