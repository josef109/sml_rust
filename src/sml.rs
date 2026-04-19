use chrono::{Local, NaiveDate};
use rumqttc::{AsyncClient, QoS};
//use std::f32::consts::PI;
use std::io::Read;
//use sml_rs::transport::SmlMessages;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, Level};
// use sml_rs::parser::ParseError;
// use sml_rs::parser::common::Value;
use sml_rs::parser::complete::File as SmlFile;
// use sml_rs::parser::complete::MessageBody::CloseResponse;
// use sml_rs::parser::complete::MessageBody::GetListResponse;
use sml_rs::parser::common::Value;
// wichtig
use anyhow::Context;
use thiserror::Error;

use crate::config::get_config;
use crate::model::{self, Energy, SensorData, SharedAppState, SseData};
use crate::rrd::update_rrd;

struct BitsNStrings {
    obis: &'static [u8; 6],
    name: &'static str,
}

const OBIS_ZAEHLERSTAND: &[u8] = &[1, 0, 1, 8, 0, 255];
const OBIS_WIRKLEISTUNG: &[u8] = &[1, 0, 16, 7, 0, 255];

const OBIS: [BitsNStrings; 11] = [
    {
        BitsNStrings {
            obis: &[1, 0, 0, 0, 1, 255],
            name: &"Seriennummer",
        }
    },
    {
        BitsNStrings {
            obis: &[1, 0, 0, 0, 9, 255],
            name: &"Geräteeinzelidentifikation",
        }
    },
    {
        BitsNStrings {
            obis: &[1, 0, 1, 8, 0, 255],
            name: &"Zählerstand Bezug",
        }
    },
    {
        BitsNStrings {
            obis: &[1, 0, 1, 8, 1, 255],
            name: &"Bezug Tarif 1",
        }
    },
    {
        BitsNStrings {
            obis: &[1, 0, 1, 8, 2, 255],
            name: &"Bezug Tarif 2",
        }
    },
    {
        BitsNStrings {
            obis: &[1, 0, 16, 7, 0, 255],
            name: &"Leistung",
        }
    },
    {
        BitsNStrings {
            obis: &[1, 0, 36, 7, 0, 255],
            name: &"Leistung an L1",
        }
    },
    {
        BitsNStrings {
            obis: &[1, 0, 56, 7, 0, 255],
            name: &"Leistung an L2",
        }
    },
    {
        BitsNStrings {
            obis: &[1, 0, 76, 7, 0, 255],
            name: &"Leistung an L3",
        }
    },
    {
        BitsNStrings {
            obis: &[129, 129, 199, 130, 3, 255],
            name: &"Herstelleridentifikation",
        }
    },
    {
        BitsNStrings {
            obis: &[129, 129, 199, 130, 5, 255],
            name: &"Public Key",
        }
    },
];

#[derive(Error, Debug)]
pub enum SmlError {
    #[error("Serieller Port Fehler: {0}")]
    SerialPort(#[from] serialport::Error),

    #[error("IO Fehler: {0}")]
    Io(#[from] std::io::Error),

    #[error("SML Dekodierungsfehler")]
    Decode,

    #[error("SML Parsingfehler: {0}")]
    Parse(String),

    #[error("Ungültiges Format in der Historien-Datei: {0}")]
    HistoryFormat(String),
}

pub async fn run_serial_loop(
    app_state: SharedAppState,
    mqtt_client: AsyncClient,
    token: CancellationToken,
) {
    let config = get_config();
    let mut sensor = SensorData::new(load_initial_values());
    let mut decoder = sml_rs::transport::Decoder::<Vec<u8>>::new();

    while !token.is_cancelled() {
        // Port öffnen mit Kontext
        let mut port = match serialport::new(&config.serial_port, 9600)
            .timeout(Duration::from_secs(5))
            .open()
        {
            Ok(p) => p,
            Err(e) => {
                error!(
                    "Serial Port {} Fehler: {}. Retry in 5s...",
                    config.serial_port, e
                );
                tokio::select! {
                    _ = sleep(Duration::from_secs(5)) => continue,
                    _ = token.cancelled() => break,
                }
            }
        };

        let mut serial_buf = [0u8; 1024]; // Größerer Puffer für Effizienz

        loop {
            if token.is_cancelled() {
                return;
            }

            match port.read(&mut serial_buf) {
                Ok(n) if n > 0 => {
                    for &byte in &serial_buf[..n] {
                        match decoder.push_byte(byte) {
                            Ok(Some(decoded_bytes)) => {
                                match sml_rs::parser::complete::parse(decoded_bytes) {
                                    Ok(message) => {
                                        process_sml_messages(
                                            Some(message),
                                            &mut sensor,
                                            &mqtt_client,
                                            &app_state,
                                        )
                                        .await;
                                    }
                                    Err(e) => error!("SML Parse Fehler: {:?}", e),
                                }
                            }
                            Ok(None) => {} // Frame noch nicht komplett
                            Err(e) => {
                                error!("SML Decode Fehler: {:?}.", e);
                            }
                        }
                    }
                }
                Ok(_) => {} // Timeout oder 0 Bytes
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    error!(
                        "Hardware-Lesefehler am Port: {:?}. Versuche Reconnect...",
                        e
                    );
                    break; // Inneren Loop verlassen -> Reconnect
                }
            }
        }
    }
}

// HIER IST DIE KORRIGIERTE FUNKTION:
async fn process_sml_messages(
    messages: Option<SmlFile<'_>>,
    sensor: &mut SensorData,
    client: &AsyncClient,
    app_state: &SharedAppState,
) {
    let mut found_data = false;

    if let Some(sml_file) = messages {
        for msg in sml_file.messages {
            if let sml_rs::parser::complete::MessageBody::GetListResponse(list_response) =
                msg.message_body
            {
                for val in list_response.val_list {
                    if tracing::event_enabled!(Level::INFO) {
                        // OBIS Code prüfen
                        for ob in OBIS {
                            if val.obj_name == ob.obis.as_ref() {
                                let v: String = match val.value {
                                    Value::I32(i) => i.to_string(),
                                    Value::I64(i) => i.to_string(),
                                    Value::Bytes(b) => {
                                        b.iter().map(|b| format!("{:02x}", b)).collect::<String>()
                                    }
                                    Value::List(_) => "".to_string(),
                                    _ => "".to_string(),
                                };
                                info!("get: {} {}", ob.name, v);
                                //error!("get: {} {}", ob.name, v);
                            }
                        }
                    }
                    if val.obj_name == OBIS_ZAEHLERSTAND {
                        match val.value {
                            Value::I64(v) => {
                                update_import(sensor, v as u64);
                                found_data = true;
                            }
                            Value::U64(v) => {
                                update_import(sensor, v);
                                found_data = true;
                            }
                            _ => {}
                        }
                    } else if val.obj_name == OBIS_WIRKLEISTUNG {
                        match val.value {
                            Value::I64(v) => {
                                sensor.power = v as i32;
                                found_data = true;
                            }
                            Value::I32(v) => {
                                sensor.power = v;
                                found_data = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    if found_data {
        handle_logic_update(sensor, client, app_state).await;
    }
}

fn update_import(sensor: &mut SensorData, val: u64) {
    let new = model::Energy(val);

    if sensor.import_old == model::Energy(0) {
        sensor.import_old = new;
    }

    let diff = new.0.saturating_sub(sensor.import_old.0);
    sensor.import_diff = diff as u32;

    sensor.import_old = new;
    sensor.import = new;
}

async fn handle_logic_update(
    sensor: &mut SensorData,
    client: &AsyncClient,
    app_state: &SharedAppState,
) {
    let config = get_config(); // Holt sich die Referenz direkt vom "Himmel"
    if sensor.power < -500 && !sensor.export_sts {
        sensor.export_sts = true;
        let _ = client
            .publish(
                "homeassistant/binary_sensor/sml/feed/state",
                QoS::AtLeastOnce,
                true,
                "ON",
            )
            .await;
    } else if sensor.power > -100 && sensor.export_sts {
        sensor.export_sts = false;
        let _ = client
            .publish(
                "homeassistant/binary_sensor/sml/feed/state",
                QoS::AtLeastOnce,
                true,
                "OFF",
            )
            .await;
    }

    let now = Instant::now();
    let w = sensor.power;
    if let Some(last_time) = sensor.last_integration_time {
        let dt = now.duration_since(last_time).as_millis() as u64;
        if w < 0 || sensor.power_old < 0 {
            let p_avg_tenths = ((-sensor.power_old).max(0) as u64 + (-w).max(0) as u64) / 2;
            sensor.export += model::HighResEnergy(p_avg_tenths * dt);
        }
    }

    sensor.last_integration_time = Some(now);
    sensor.power_old = sensor.power;

    // Diff wird in High-Res berechnet
    sensor.export_diff = sensor.export.0.saturating_sub(sensor.export_old.0);
    sensor.export_old = sensor.export;

    let rrd_import = sensor.import;
    // Einfach in Standard-Energy umwandeln
    let rrd_export = sensor.export.to_energy();
    let rrd_power = sensor.power;

    tokio::task::spawn_blocking(move || {
        if let Err(e) = update_rrd(rrd_import.0, rrd_export.0, rrd_power) {
            error!("RRD Update fehlgeschlagen: {:?}", e);
        }
    });

    info!(
        "Bezug: {} Einspeisung: {} Wirkleistung: {}",
        sensor.import,
        sensor.export.to_energy(),
        sensor.power as f32 / 10.0
    );

    let should_publish = match sensor.last_mqtt_publish {
        Some(t) => now.duration_since(t).as_secs() > 60,
        None => true,
    };

    if should_publish {
        let json_payload = format!(
            "{{\"Time\":\"{}\",\"bezug\":{},\"einspeisung\":{}}}",
            Local::now().to_rfc3339(),
            sensor.import.0,
            sensor.export.to_energy().0
        );
        let _ = client
            .publish(
                "homeassistant/sensor/sml/zaehler/state",
                QoS::AtLeastOnce,
                false,
                json_payload,
            )
            .await;
        sensor.last_mqtt_publish = Some(now);
    }

    let _ = client
        .publish(
            "homeassistant/sensor/sml/wirkleistung/state",
            QoS::AtLeastOnce,
            false,
            sensor.power.to_string(),
        )
        .await;

    match app_state.lock() {
        Ok(state) => {
            let _ = state.tx.send(SseData {
                time: Local::now(),
                power: sensor.power,
                import: sensor.import,
                import_diff: sensor.import_diff as f32,
                export: sensor.export.to_energy(),

                // Diff umrechnen: Da export_diff in HighRes (u64) ist, skalieren wir es runter.
                // Das geteilt durch 10 bringt es von 0.1 Wh auf echte Wh.
                export_diff: (sensor.export_diff as f32
                    / model::HighResEnergy::SCALE_FACTOR as f32)
                    / 10.0,

                is_feed_in: sensor.export_sts,
            });
        }
        Err(e) => error!("App State Mutex Poisoned: {}", e),
    }

    let now = Local::now();
    let today = now.date_naive();

    if sensor.last_daily_log != today {
        let path = &config.daily_log_path;

        let _result: anyhow::Result<()> = (|| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file_exists = path.exists();
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;

            if !file_exists {
                writeln!(file, "Datum;Bezug_Wh;Einspeisung_Wh")?;
            }

            writeln!(
                file,
                "{};{};{}",
                today,
                sensor.import,
                sensor.export.to_energy()
            )?;
            Ok(())
        })();
    }
}

pub fn load_initial_values() -> Energy {
    // Wir versuchen zu laden, bei jedem Fehler loggen wir ihn und starten mit 0
    match try_load_history() {
        Ok(export) => export,
        Err(e) => {
            info!("Starte mit Initialwert 0 (Grund: {})", e);
            model::Energy(0)
        }
    }
}

fn try_load_history() -> anyhow::Result<Energy> {
    let config = get_config();
    let path = PathBuf::from(&config.daily_log_path);

    if !path.exists() {
        anyhow::bail!("Datei existiert noch nicht");
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Fehler beim Lesen der Datei {:?}", path))?;

    let last_line = content
        .lines()
        .rfind(|l| !l.trim().is_empty())
        .context("Datei ist leer oder enthält nur Leerzeilen")?;

    if last_line.starts_with("Datum") {
        return Ok(model::Energy(0));
    }

    let parts: Vec<&str> = last_line.split(';').collect();
    if parts.len() < 3 {
        return Err(
            SmlError::HistoryFormat(format!("Zu wenig Spalten in Zeile: '{}'", last_line)).into(),
        );
    }

    let export = parts[2]
        .parse::<u64>()
        .with_context(|| format!("Ungültiger Export-Wert in Zeile: '{}'", last_line))?;

    info!("Historie geladen: Export steht bei {}", export);

    Ok(model::Energy(export))
}
