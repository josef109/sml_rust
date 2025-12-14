use chrono::Local;
use rumqttc::{AsyncClient, QoS};
//use std::f32::consts::PI;
use std::io::Read;
//use sml_rs::transport::SmlMessages;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{str, string};

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

use crate::config::Config;
use crate::model::{SensorData, SharedAppState, SseData};
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

pub async fn run_serial_loop(
    config: Config,
    app_state: SharedAppState,
    mqtt_client: AsyncClient,
    token: CancellationToken,
) {
    let mut sensor = SensorData::new();

    // let buf = ArrayBuf::<4069>::default();
    // let mut decoder = sml_rs::transport::Decoder::from_buf(buf);

    let mut decoder = sml_rs::transport::Decoder::<Vec<u8>>::new();

    info!("Starting SML Reader Loop on {}", config.serial_port);

    loop {
        if token.is_cancelled() {
            break;
        }
        let mut port = match serialport::new(&config.serial_port, 9600)
            .timeout(Duration::from_secs(5))
            .open()
        {
            Ok(p) => {
                info!("Serial interface {} opened", config.serial_port);
                p
            }
            Err(e) => {
                error!(
                    "Error opening {}: {}. Retrying in 5s...",
                    config.serial_port, e
                );
                tokio::select! {
                    // Option 1: Warte 30 Sekunden
                    _ =  sleep(Duration::from_secs(5)) => {
                        // Führe nach dem Sleep den Haupt-Code aus
                    }
                    // Option 2: Warte auf das Abbruch-Token
                    _ = token.cancelled() => {
                        info!("Graph loop received cancellation signal. Exiting.");
                        break; // Schleife verlassen und Funktion beenden
                    }
                }
                continue;
            }
        };

        let mut serial_buf = [0u8; 256];

        loop {
            if token.is_cancelled() {
                info!("Close Port {}...", config.serial_port);
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
                                            &config.rrd_path,
                                        )
                                        .await;
                                    }
                                    Err(e) => {
                                        error!("Parsing error: {:?}", e);
                                        //None
                                    }
                                };
                            }
                            Ok(None) => {}
                            Err(e) => {
                                error!("Decode Error: {:?}", e);
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    error!("Serial Port Read Error: {:?}", e);
                    break;
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
    }
}

// HIER IST DIE KORRIGIERTE FUNKTION:
async fn process_sml_messages(
    messages: Option<SmlFile<'_>>,
    sensor: &mut SensorData,
    client: &AsyncClient,
    app_state: &SharedAppState,
    rrd_path: &Path,
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
                                update_zaehlerstand(sensor, v as u64);
                                found_data = true;
                            }
                            Value::U64(v) => {
                                update_zaehlerstand(sensor, v);
                                found_data = true;
                            }
                            _ => {}
                        }
                    } else if val.obj_name == OBIS_WIRKLEISTUNG {
                        match val.value {
                            Value::I64(v) => {
                                sensor.wirkleistung = v as i32;
                                found_data = true;
                            }
                            Value::I32(v) => {
                                sensor.wirkleistung = v;
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
        handle_logic_update(sensor, client, app_state, rrd_path).await;
    }
}

fn update_zaehlerstand(sensor: &mut SensorData, val: u64) {
    if sensor.zaehlerstand_alt == 0 {
        sensor.zaehlerstand_alt = val;
    }
    sensor.zaehlerstand_diff = (val - sensor.zaehlerstand_alt) as u32;
    sensor.zaehlerstand_alt = val;
    sensor.zaehlerstand = val;
}

async fn handle_logic_update(
    sensor: &mut SensorData,
    client: &AsyncClient,
    app_state: &SharedAppState,
    rrd_path: &Path,
) {
    if sensor.wirkleistung < -500 && !sensor.einspeisung_sts {
        sensor.einspeisung_sts = true;
        let _ = client
            .publish(
                "homeassistant/binary_sensor/sml/feed/state",
                QoS::AtLeastOnce,
                true,
                "ON",
            )
            .await;
    } else if sensor.wirkleistung > -100 && sensor.einspeisung_sts {
        sensor.einspeisung_sts = false;
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
    let w = sensor.wirkleistung; //-sensor.wirkleistung;
    if let Some(last_time) = sensor.last_integration_time {
        let dt = now.duration_since(last_time).as_millis() as u32;
        if w < 0 || sensor.wirkleistung_alt < 0 {
            let p_avg = (((-sensor.wirkleistung_alt).max(0) + (-w).max(0)) / 2) as u32;
            sensor.einspeisung += p_avg as u64 * dt as u64; // 1/10 W * ms     ms 1000  3600 h    // / 360.0; // 1/10 Wh
        }
    }
    sensor.last_integration_time = Some(now);
    sensor.wirkleistung_alt = sensor.wirkleistung;

    update_rrd(
        rrd_path,
        sensor.zaehlerstand,
        sensor.einspeisung / 2000000,
        sensor.wirkleistung,
    );

    info!(
        "Bezug: {} Einspeisung: {} Wirkleistung: {}",
        sensor.zaehlerstand as f64 / 10.0,
        sensor.einspeisung as f32 / 36000000.0,
        sensor.wirkleistung as f32 / 10.0
    );

    let should_publish = match sensor.last_mqtt_publish {
        Some(t) => now.duration_since(t).as_secs() > 60,
        None => true,
    };

    if should_publish {
        let json_payload = format!(
            "{{\"Time\":\"{}\",\"bezug\":{}.{},\"einspeisung\":{}.{}}}",
            Local::now().to_rfc3339(),
            sensor.zaehlerstand / 10,
            sensor.zaehlerstand % 10,
            sensor.einspeisung / 36000000,
            sensor.einspeisung % 36000000 / 3600000
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
            (sensor.wirkleistung / 10).to_string() + "." + &(sensor.wirkleistung % 10).to_string(),
        )
        .await;

    match app_state.lock() {
        Ok(mut state) => {
            state.wirkleistung = sensor.wirkleistung as f32 / 10.0;
            state.zaehlerstand_diff = sensor.zaehlerstand_diff as f32 / 10.0;
            let _ = state.tx.send(SseData {
                time: Local::now().format("%H:%M:%S").to_string(),
                value: sensor.wirkleistung as f32 / 10.0,
                value2: sensor.zaehlerstand_diff as f32 / 10.0,
                total_energy: sensor.zaehlerstand as f64 / 10.0,
                is_feed_in: sensor.einspeisung_sts,
            });
        }
        Err(e) => {
            error!("App State Mutex Poisoned: {}", e);
        }
    }
}
