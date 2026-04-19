use chrono::{DateTime, Local, NaiveDate};
use serde::{Serialize, Serializer};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::broadcast;

use crate::model;

#[derive(Clone, Serialize, Debug)]
pub struct SseData {
    pub time: DateTime<Local>,
    pub power: i32,
    pub import: Energy, // Automatisch f32 im JSON
    pub import_diff: f32,
    pub export: Energy, // Automatisch f32 im JSON
    pub export_diff: f32,
    pub is_feed_in: bool,
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
    pub import: Energy,
    pub import_old: Energy,
    pub import_diff: u32,

    pub export: HighResEnergy,
    pub export_old: HighResEnergy,
    pub export_diff: u64,

    pub export_sts: bool,
    pub last_integration_time: Option<Instant>,
    pub last_mqtt_publish: Option<Instant>,
    pub last_daily_log: NaiveDate,
}

impl SensorData {
    pub fn new(initial_export: Energy) -> Self {
        // Initiale Konvertierung von Standard-Auflösung in High-Res
        let initial_high_res = HighResEnergy::from_energy(initial_export);

        Self {
            power: 0,
            power_old: 0,
            import: model::Energy(0),
            import_old: model::Energy(0),
            import_diff: 0,
            export: initial_high_res,
            export_old: initial_high_res,
            export_diff: 0,
            export_sts: true,
            last_integration_time: None,
            last_mqtt_publish: None,
            last_daily_log: Local::now().date_naive(),
        }
    }
}

use derive_more::{Add, AddAssign, Div, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, Sub, AddAssign, Mul, Div)]
pub struct Energy(pub u64); // Intern in 0.1 Wh

impl Energy {
    // Statt f32 bei der Anzeige:
    pub fn to_wh_string(&self) -> String {
        format!("{}.{} Wh", self.0 / 10, self.0 % 10)
    }
}

// Automatische Konvertierung beim Erstellen
impl From<u64> for Energy {
    fn from(v: u64) -> Self {
        Self(v)
    }
}
impl From<f32> for Energy {
    fn from(v: f32) -> Self {
        Self((v * 10.0).round() as u64)
    }
}

// Trait für die Rückgabe
pub trait FromEnergy {
    fn from_energy(raw: u64) -> Self;
}
impl FromEnergy for f32 {
    fn from_energy(raw: u64) -> Self {
        raw as f32 / 10.0
    }
}
impl FromEnergy for u64 {
    fn from_energy(raw: u64) -> Self {
        raw
    }
}
use std::fmt;

impl fmt::Display for Energy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = self.0;
        let whole = raw / 10;
        let frac = raw % 10;
        write!(f, "{}.{}", whole, frac)
    }
}

impl Serialize for Energy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Wir senden den Wert als Ganzzahl (in 0.1 Wh Einheiten).
        serializer.serialize_u64(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, Sub, AddAssign, Mul, Div)]
pub struct HighResEnergy(pub u64); // Speichert die rohen Integrationswerte (W*ms)

impl HighResEnergy {
    // Zentral definierter Skalierungsfaktor
    // Hinweis: W*ms in Wh ist physikalisch Faktor 3.600.000.
    pub const SCALE_FACTOR: u64 = 2_000_000;

    // Erstellt einen hochauflösenden Wert aus einem Basis-Energy-Wert (z.B. beim Laden der Historie)
    pub fn from_energy(energy: Energy) -> Self {
        Self(energy.0 * Self::SCALE_FACTOR)
    }

    // Rechnet den internen Wert auf die Standard-0.1-Wh-Auflösung herunter
    pub fn to_energy(self) -> Energy {
        Energy(self.0 / Self::SCALE_FACTOR)
    }
}
