use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::broadcast;

#[derive(Clone, Serialize, Debug)]
pub struct SseData {
    pub time: String,
    pub value: f32,        // Aktuelle Leistung (Watt)
    pub value2: f32,       // Differenz (für Chart)
    pub total_energy: f64, // NEU: Zählerstand Total (kWh)
    pub is_feed_in: bool,  // NEU: Status Einspeisung
}

pub struct AppState {
    pub wirkleistung: f32,
    pub zaehlerstand_diff: f32,
    pub einspeisung: f32,
    pub einspeisung_sts: bool,
    pub tx: broadcast::Sender<SseData>,
}

pub type SharedAppState = Arc<Mutex<AppState>>;

pub struct SensorData {
    pub wirkleistung: i32,
    pub wirkleistung_alt: i32,
    pub zaehlerstand: u64,
    pub zaehlerstand_alt: u64,
    pub zaehlerstand_diff: u32,
    pub einspeisung: u64,
    pub einspeisung_sts: bool,
    pub last_integration_time: Option<Instant>,
    pub last_mqtt_publish: Option<Instant>,
    //pub sin: f32,
}

impl SensorData {
    pub fn new() -> Self {
        Self {
            wirkleistung: 0,
            wirkleistung_alt: 0,
            zaehlerstand: 0,
            zaehlerstand_alt: 0,
            zaehlerstand_diff: 0,
            einspeisung: 0,
            einspeisung_sts: true,
            last_integration_time: None,
            last_mqtt_publish: None,
            // sin: 0.0,
        }
    }
}
