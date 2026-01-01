use chrono::{DateTime, Local, NaiveDate};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::broadcast;

#[derive(Clone, Serialize, Debug)]
pub struct SseData {
    pub time: DateTime<Local>,
    pub power: i32,       // Aktuelle Leistung (Watt)
    pub import: u64,      // Zählerstand Total (kWh)
    pub import_diff: u32, // Differenz (für Chart)
    pub export: u64,      // Einspeisung total
    pub export_diff: u32, // Differenz (für Chart)
    pub is_feed_in: bool, // Status Einspeisung
}

pub struct AppState {
    // pub power: f32,
    // pub import_diff: f32,
    // pub export: f32,
    // pub export_sts: bool,
    pub tx: broadcast::Sender<SseData>,
    //     pub default_language: String,
    //     pub rrd_path: PathBuf,
    //     pub daily_log_path: PathBuf,
}

pub type SharedAppState = Arc<Mutex<AppState>>;

pub struct SensorData {
    pub power: i32,
    pub power_old: i32,
    pub import: u64,
    pub import_old: u64,
    pub import_diff: u32,
    pub export: u64,
    pub export_old: u64,
    pub export_diff: u32,
    pub export_sts: bool,
    pub last_integration_time: Option<Instant>,
    pub last_mqtt_publish: Option<Instant>,
    pub last_daily_log: NaiveDate,
    //pub sin: f32,
}

impl SensorData {
    pub fn new(initial_export: u64) -> Self {
        Self {
            power: 0,
            power_old: 0,
            import: 0,
            import_old: 0,
            import_diff: 0,
            export: initial_export,
            export_old: initial_export,
            export_diff: 0,
            export_sts: true,
            last_integration_time: None,
            last_mqtt_publish: None,
            last_daily_log: Local::now().date_naive(),
            // sin: 0.0,
        }
    }
}
