use crate::config::Config;
use crate::model::SharedAppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Local, TimeZone};
//use chrono::format::Numeric;
use chrono::{Datelike, Months, NaiveTime, Timelike, Utc};
use rrd::ops::fetch;
use serde::Deserialize;
use tokio::time::sleep;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use rrd::ops::graph::elements::{AreaColor, ColorWithLegend, Legend};
use rrd::ops::graph::props::{Labels, Size, UnitsExponent};
use rrd::ops::graph::{self, elements, props};
//use rrd::ConsolidationFn;
use rrd::{
    ops::{create, graph::elements::VarName, graph::props::ImageFormat, update},
    ConsolidationFn,
};
use std::error::Error;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

type UtcDateTime = chrono::DateTime<chrono::Utc>;

#[derive(Deserialize)]
pub struct LiveQuery {
    pub ds: String,
    pub seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Language {
    De,
    En,
}

pub fn save_rrd_on_shutdown(config: &Config) {
    if Path::new(&config.rrd_path).exists() {
        match std::fs::copy(&config.rrd_path, &config.rrd_backup_path) {
            Ok(bytes_copied) => {
                info!(
                    "RRD database backup successful. Copied {} bytes.",
                    bytes_copied
                );
            }
            Err(e) => {
                error!("Error backing up RRD database: {}", e);
            }
        }
    }
}

pub fn ensure_rrd(config: &Config) {
    if !Path::new(&config.rrd_path).exists() {
        // file present?
        if let Some(parent_dir) = Path::new(&config.rrd_path).parent() {
            // 2. Check if the parent directory exists
            if !parent_dir.exists() {
                // 3. Create the directory (and any necessary parents)
                if let Err(e) = std::fs::create_dir_all(parent_dir) {
                    // Handle the error if directory creation fails
                    error!(
                        "Failed to create RRD directory {}: {}",
                        parent_dir.display(),
                        e
                    );
                }
            }
        }

        if Path::new(&config.rrd_backup_path).exists() {
            info!("Restoring RRD from backup...");
            match std::fs::copy(&config.rrd_backup_path, &config.rrd_path) {
                Ok(bytes_copied) => {
                    info!(
                        "RRD database backup restored. Copied {} bytes.",
                        bytes_copied
                    );
                }
                Err(e) => {
                    error!("Error restoring up RRD database: {}", e);
                }
            }
        } else {
            info!("Creating new RRD database: {}", config.rrd_path.display());
            let _ = create::create(
                &config.rrd_path,
                chrono::Utc::now(),
                Duration::from_secs(5),
                false,
                None,
                &[],
                &[
                    create::DataSource::counter(
                        create::DataSourceName::new("Bezug"),
                        900,
                        Some(0),
                        Some(10000000000),
                    ),
                    create::DataSource::counter(
                        create::DataSourceName::new("Einspeisung"),
                        900,
                        Some(0),
                        Some(10000000000),
                    ),
                    create::DataSource::gauge(
                        create::DataSourceName::new("Wirkleistung"),
                        900,
                        Some(-10000.0),
                        Some(100000.0),
                    ),
                ],
                &[
                    create::Archive::new(ConsolidationFn::Avg, 0.5, 1, 720 * 24).unwrap(),
                    create::Archive::new(ConsolidationFn::Avg, 0.5, 720, 8760 * 2).unwrap(),
                ],
            );
        }
    }
}

pub async fn live_data(
    State(state): State<SharedAppState>,
    Query(q): Query<LiveQuery>,
) -> Json<Vec<(DateTime<Local>, i32)>> {
    let rrd_path = {
        let s = state.lock().unwrap();
        s.rrd_path.clone()
    };

    let now = chrono::Utc::now();
    let start = now - Duration::from_secs(q.seconds);

    // Fetch ausführen
    let rc = fetch::fetch(
        &rrd_path,
        ConsolidationFn::Avg,
        start,
        now,
        Duration::from_secs(3),
    );

    match rc {
        Ok(data) => {
            let ds_names = data.ds_names();
            // Index der gewünschten Datenquelle (z.B. "Wirkleistung") finden
            let ds_index = ds_names.iter().position(|name| name == &q.ds);

            if let Some(idx) = ds_index {
                let points: Vec<(DateTime<Local>, i32)> = data
                    .rows()
                    .iter()
                    .filter_map(|row| {
                        let val = row[idx];
                        // NaN Werte (Lücken in der RRD) ignorieren
                        if val.is_nan() {
                            None
                        } else {
                            let ts = row.timestamp().timestamp();
                            // 2. In lokale Zeit umwandeln (wie in sml.rs)
                            let local_time = chrono::Local.timestamp_opt(ts, 0).unwrap();

                            Some((local_time, val as i32))
                        }
                    })
                    .collect();

                Json(points)
            } else {
                Json(vec![]) // DS nicht gefunden
            }
        }
        Err(err) => {
            error!("RRD Fetch Fehler: {}", err);
            Json(vec![])
        }
    }
}

pub fn update_rrd(path: &Path, import: u64, export: u64, power: i32) {
    let rc = update::update_all(
        path,
        update::ExtraFlags::empty(),
        &[(
            update::BatchTime::Now,
            &[import.into(), export.into(), (power as f64).into()],
        )],
    );
    match rc {
        Ok(_) => info!("Ok"),
        Err(err) => error!("Not ok: {err}"),
    }
}

pub async fn run_graph_loop(config: Config, token: CancellationToken) {
    let mut last_hour = Local::now().hour();
    let mut last_month = Local::now().month();
    let mut first_loop = false;
    let lang_enum = if config.language == "en" {
        Language::En
    } else {
        Language::De
    };
    info!("Starting native graph generator loop");

    if !Path::new(&config.image_output_path).exists() {
        let _ = std::fs::create_dir_all(&config.image_output_path);
    }

    loop {
        tokio::select! {
            // Option 1: Warte 30 Sekunden
            _ = sleep(Duration::from_secs(30)) => {
                // Führe nach dem Sleep den Haupt-Code aus
            }
            // Option 2: Warte auf das Abbruch-Token
            _ = token.cancelled() => {
                info!("Graph loop received cancellation signal. Exiting.");
                break; // Schleife verlassen und Funktion beenden
            }
        }
        let now = Local::now();
        let current_hour = now.hour();
        let current_month = now.month();

        if let Err(e) = generate_graph(
            config.rrd_path.clone(),
            &config.image_output_path,
            // &path_hour,
            GraphPeriod::Hour,
            lang_enum,
        ) {
            error!("Error generating hourly graph: {}", e);
        }

        if current_hour != last_hour || !first_loop {
            if current_hour == 0 {
                info!("Backing up RRD database");
                match std::fs::copy(&config.rrd_path, &config.rrd_backup_path) {
                    Ok(bytes_copied) => {
                        info!(
                            "RRD database backup successful. Copied {} bytes.",
                            bytes_copied
                        );
                    }
                    Err(e) => {
                        error!("Error backing up RRD database: {}", e);
                    }
                }
            }
            info!("Generating day graph");

            if let Err(e) = generate_graph(
                config.rrd_path.clone(),
                &config.image_output_path,
                GraphPeriod::Day,
                lang_enum,
            ) {
                error!("Error generating daily graph (DE): {}", e);
            }

            if current_hour == 1 || !first_loop {
                info!("Generating week graph");

                if let Err(e) = generate_graph(
                    config.rrd_path.clone(),
                    &config.image_output_path,
                    GraphPeriod::Week,
                    lang_enum,
                ) {
                    error!("Error generating daily graph (DE): {}", e);
                }
            }

            last_hour = current_hour;
        }
        if (current_month != last_month && current_hour == 2) || !first_loop {
            info!("Generating month graph");

            if let Err(e) = generate_graph(
                config.rrd_path.clone(),
                &config.image_output_path,
                GraphPeriod::Month,
                lang_enum,
            ) {
                error!("Error generating daily graph (DE): {}", e);
            }

            last_month = current_month;
        }
        first_loop = true;
    }
}

#[derive(Debug, Clone, Copy)]
enum GraphPeriod {
    Hour,
    Day,
    Week,
    Month,
}

fn get_graph_range(period: GraphPeriod) -> (UtcDateTime, Option<UtcDateTime>) {
    let now = Utc::now();
    let today_midnight = now.with_time(NaiveTime::MIN).unwrap() - Duration::from_secs(1);

    match period {
        GraphPeriod::Hour => (now - Duration::from_hours(1) + Duration::from_secs(1), None),
        GraphPeriod::Day => (
            now - Duration::from_hours(24) + Duration::from_secs(1),
            None,
        ),
        GraphPeriod::Week => {
            // Start: Vor 7 Tagen um 00:00
            // Ende: Heute um 00:00
            let start = (now - Duration::from_hours(24 * 7))
                .with_time(NaiveTime::MIN)
                .unwrap();
            (start, Some(today_midnight))
        }
        GraphPeriod::Month => {
            // Start: Der 1. des Vormonats um 00:00
            let last_month_root = now.checked_sub_months(Months::new(1)).expect("Date error");
            let start = last_month_root
                .with_day(1)
                .unwrap()
                .with_time(NaiveTime::MIN)
                .unwrap();

            // Ende: Der letzte Moment des Vormonats (01. d. M. 00:00:00 - 1 Sekunde)
            let end = now.with_day(1).unwrap().with_time(NaiveTime::MIN).unwrap()
                - Duration::from_secs(1);

            (start, Some(end))
        }
    }
}

fn generate_graph(
    rrd_file: PathBuf,
    output_path: &str,
    period: GraphPeriod,
    lang: Language,
) -> Result<(), Box<dyn Error>> {
    let watermark = Local::now().format("%Y-%m-%d %H\\:%M\\:%S").to_string();

    let (label_title, label_import, label_export) = match (period, &lang) {
        (GraphPeriod::Hour, Language::De) => {
            ("Stromverbrauch - Letzte Stunde", "Bezug", "Einspeisung")
        }
        (GraphPeriod::Hour, Language::En) => ("Power Usage - Last Hour", "Import", "Export"),
        (GraphPeriod::Day, Language::De) => {
            ("Stromverbrauch - Letzte 24 Sunden", "Bezug", "Einspeisung")
        }
        (GraphPeriod::Day, Language::En) => ("Power Usage - this day", "Import", "Export"),
        (GraphPeriod::Week, Language::De) => {
            ("Stromverbrauch - diese Woche", "Bezug", "Einspeisung")
        }
        (GraphPeriod::Week, Language::En) => ("Power Usage - this week", "Import", "Export"),
        (GraphPeriod::Month, Language::De) => {
            ("Stromverbrauch - letzten Monat", "Bezug", "Einspeisung")
        }
        (GraphPeriod::Month, Language::En) => ("Power Usage - last month", "Import", "Export"),
    };

    let output_file = output_path.to_owned()
        + "/power_"
        + match period {
            GraphPeriod::Hour => "hour",
            GraphPeriod::Day => "day",
            GraphPeriod::Week => "week",
            GraphPeriod::Month => "month",
        }
        + ".png";
    // Die Dauer, um die zurückgerechnet werden soll, basierend auf der Enum ermitteln
    //let start_time = Utc::now() - calculate_duration(period);
    let (start_time, end_time) = get_graph_range(period);
    debug!("Start: {} Ende: {:?}", start_time, end_time);
    // let start_time = match period {
    //     GraphPeriod::Hour => Utc::now() - Duration::from_secs(60 * 60),
    //     GraphPeriod::Day => Utc::now() - Duration::from_secs(60 * 60 * 24),
    //     GraphPeriod::Week => {
    //         (Utc::now() - Duration::days(7))
    //             .with_time(chrono::NaiveTime::MIN) // Setzt auf 00:00:00
    //             .unwrap()
    //     }
    //     GraphPeriod::Month => Utc::now()
    //         .checked_sub_months(Months::new(1))
    //         .expect("Date error"),
    // };
    // let end_time = match period {
    //     GraphPeriod::Hour => None,
    //     GraphPeriod::Day => None,
    //     GraphPeriod::Week => Some(Utc::now() - Duration::from_secs(60 * 60 * 24 * 7)),
    //     GraphPeriod::Month => Some(
    //         Utc::now()
    //             .checked_sub_months(Months::new(1))
    //             .expect("Date error"),
    //     ),
    // };
    // let var_name_ein = VarName::new("ein".to_string())?;
    // let var_name_bez = VarName::new("bez".to_string())?;
    // let var_name_lei = VarName::new("lei".to_string())?;

    let graph_elements = vec![
        elements::Def {
            var_name: VarName::new("ein".to_string())?,
            rrd: rrd_file.clone(),
            ds_name: "Einspeisung".to_string(),
            consolidation_fn: ConsolidationFn::Avg,
            step: None,
            start: None,
            end: None,
            reduce: None,
        }
        .into(),
        elements::Def {
            var_name: VarName::new("bez".to_string())?,
            rrd: rrd_file.clone(),
            ds_name: "Bezug".to_string(),
            consolidation_fn: ConsolidationFn::Avg,
            step: None,
            start: None,
            end: None,
            reduce: None,
        }
        .into(),
        elements::Def {
            var_name: VarName::new("lei".to_string())?,
            rrd: rrd_file,
            ds_name: "Wirkleistung".to_string(),
            consolidation_fn: ConsolidationFn::Avg,
            step: None,
            start: None,
            end: None,
            reduce: None,
        }
        .into(),
        elements::CDef {
            var_name: VarName::new("einspeisung".to_string())?,
            rpn: "ein,36,*".to_string(),
        }
        .into(),
        elements::CDef {
            var_name: VarName::new("bezug".to_string())?,
            rpn: "bez,36,*".to_string(),
        }
        .into(),
        elements::CDef {
            var_name: VarName::new("bezug_kwh".to_string())?,
            rpn: "bez,10000,/".to_string(),
        }
        .into(),
        elements::CDef {
            var_name: VarName::new("wirkleistung".to_string())?,
            rpn: "lei,10000,+,100,/".to_string(),
        }
        .into(),
        elements::VDef {
            var_name: VarName::new("Verbrauch".to_string())?,
            rpn: "bezug_kwh,TOTAL".to_string(),
        }
        .into(),
        elements::Line {
            width: 5.0,
            value: VarName::new("bezug".to_string())?,
            color: Some(ColorWithLegend {
                color: "#00FF00".parse()?,
                legend: Some(Legend::from(label_import.to_string())),
            }),
            stack: false,
            skip_scale: false,
            dashes: None,
        }
        .into(),
        elements::Area {
            value: VarName::new("bezug".to_string())?,
            color: Some(ColorWithLegend {
                color: AreaColor::Color("#7FFF7FFF".parse()?),
                legend: None,
            }),
            stack: false,
            skip_scale: false,
        }
        .into(),
        elements::Line {
            width: 5.0,
            value: VarName::new("einspeisung".to_string())?,
            color: Some(ColorWithLegend {
                color: "#FF0000".parse()?,
                legend: Some(Legend::from(label_export.to_string())),
            }),
            stack: false,
            skip_scale: false,
            dashes: None,
        }
        .into(),
        elements::Area {
            value: VarName::new("einspeisung".to_string())?,
            color: Some(ColorWithLegend {
                color: AreaColor::Color("#FF7F7F7F".parse()?),
                legend: None,
            }),
            stack: false,
            skip_scale: false,
        }
        .into(),
        elements::Line {
            width: 3.0,
            value: VarName::new("wirkleistung".to_string())?,
            color: Some(ColorWithLegend {
                color: "#FFF000".parse()?,
                legend: Some(Legend::from("Wirkleistung".to_string())),
            }),
            stack: false,
            skip_scale: false,
            dashes: None,
        }
        .into(),
        elements::HRule {
            value: rrd::ops::graph::elements::Value::Constant(100.0),
            color: "#FFF000".parse().unwrap(),
            legend: None,
            dashes: None,
        }
        .into(),
        elements::GPrint {
            var_name: VarName::new("Verbrauch".to_string())?,
            format: "Verbrauch\\: %1.2lf kWh".to_string(),
        }
        .into(),
        elements::Comment { text: watermark }.into(),
    ];

    let graph_props = props::GraphProps {
        size: Size {
            width: Some(1024),
            height: Some(612),
            ..Default::default()
        },
        labels: Labels {
            title: Some(label_title.to_string()),
            vertical_label: Some("Watt (Wh)".to_string()),
        },
        time_range: props::TimeRange {
            start: Some(start_time),
            end: end_time,
            ..Default::default()
        },
        y_axis: props::YAxis {
            units_exponent: Some(UnitsExponent { exp: 0 }),
            ..Default::default()
        },
        right_y_axis: Some(props::RightYAxis {
            scale: 10.0,
            shift: -1000,
            label: Some("Leistung (W)".to_string()),
            formatter: Some(props::YAxisFormatter::Numeric),
            format: Some("%4.0lf".to_string()),
        }),
        ..Default::default()
    };

    let (image_data, _metadata) = graph::graph(ImageFormat::Png, graph_props, &graph_elements)
        .map_err(|e| format!("RRD Graph Error: {:?}", e))?;

    std::fs::write(output_file, image_data)?;
    Ok(())
}
