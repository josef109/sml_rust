use crate::config::Config;
//use chrono::format::Numeric;
use chrono::{Local, Timelike, Utc};
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
use tracing::{error, info};

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

pub fn update_rrd(path: &Path, bezug: u64, einspeisung: u64, wirkleistung: i32) {
    let rc = update::update_all(
        path,
        update::ExtraFlags::empty(),
        &[(
            update::BatchTime::Now,
            &[
                bezug.into(),
                einspeisung.into(),
                (wirkleistung as f64).into(),
            ],
        )],
    );
    match rc {
        Ok(_) => info!("Ok"),
        Err(err) => error!("Not ok: {err}"),
    }
}

pub async fn run_graph_loop(config: Config, token: CancellationToken) {
    let mut last_hour = Local::now().hour();
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

        let path_hour_de = format!("{}/strom-stunde-de.png", config.image_output_path);
        let path_hour_en = format!("{}/strom-stunde-en.png", config.image_output_path);

        if let Err(e) = generate_graph(
            config.rrd_path.clone(),
            &path_hour_de,
            GraphPeriod::Hour,
            Language::De,
        ) {
            error!("Error generating hourly graph (DE): {}", e);
        }
        if let Err(e) = generate_graph(
            config.rrd_path.clone(),
            &path_hour_en,
            GraphPeriod::Hour,
            Language::En,
        ) {
            error!("Error generating hourly graph (EN): {}", e);
        }

        if current_hour != last_hour {
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
            let path_day_de = format!("{}/strom-tag-de.png", config.image_output_path);
            let path_day_en = format!("{}/strom-tag-en.png", config.image_output_path);

            if let Err(e) = generate_graph(
                config.rrd_path.clone(),
                &path_day_de,
                GraphPeriod::Day,
                Language::De,
            ) {
                error!("Error generating daily graph (DE): {}", e);
            }
            if let Err(e) = generate_graph(
                config.rrd_path.clone(),
                &path_day_en,
                GraphPeriod::Day,
                Language::En,
            ) {
                error!("Error generating daily graph (EN): {}", e);
            }

            if current_hour == 1 {
                info!("Generating week graph");
                let path_week_de = format!("{}/strom-woche-de.png", config.image_output_path);
                let path_week_en = format!("{}/strom-week-en.png", config.image_output_path);

                if let Err(e) = generate_graph(
                    config.rrd_path.clone(),
                    &path_week_de,
                    GraphPeriod::Week,
                    Language::De,
                ) {
                    error!("Error generating daily graph (DE): {}", e);
                }
                if let Err(e) = generate_graph(
                    config.rrd_path.clone(),
                    &path_week_en,
                    GraphPeriod::Week,
                    Language::En,
                ) {
                    error!("Error generating daily graph (EN): {}", e);
                }
            }
            last_hour = current_hour;
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum GraphPeriod {
    Hour,
    Day,
    Week,
}

fn calculate_duration(period: GraphPeriod) -> Duration {
    // Definieren Sie die Umrechnungsfaktoren
    const SECONDS_PER_HOUR: u64 = 60 * 60;
    const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * 24;
    const SECONDS_PER_WEEK: u64 = SECONDS_PER_DAY * 7;

    // Berechnen Sie die Sekundenanzahl mit einem match-Statement
    let seconds = match period {
        GraphPeriod::Hour => SECONDS_PER_HOUR,
        GraphPeriod::Day => SECONDS_PER_DAY,
        GraphPeriod::Week => SECONDS_PER_WEEK,
    };

    // Erzeugen Sie die std/tokio::Duration
    Duration::from_secs(seconds)
}

fn generate_graph(
    rrd_file: PathBuf,
    output_file: &str,
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
    };

    // Die Dauer, um die zurückgerechnet werden soll, basierend auf der Enum ermitteln

    let start_time = Utc::now() - calculate_duration(period);

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
            var_name: VarName::new("wirkleistung".to_string())?,
            rpn: "lei,10000,+,100,/".to_string(),
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
