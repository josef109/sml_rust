mod config;
mod model;
mod mqtt;
mod rrd;
mod sml;
mod web;

use crate::config::Config;
use crate::model::AppState;
//use anyhow::Ok;
use ::rrd::ops::version::librrd_version;
use clap::Parser;
use std::sync::{Arc, Mutex};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let token = CancellationToken::new();
    let cloned_token = token.clone();
    let graph_token = token.clone();

    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        sigterm.recv().await;
        info!("Shutdown eingeleitet...");
        cloned_token.cancel();
    });

    let config = Config::parse();
    info!("Starting SML Service. Serial port: {}", config.serial_port);

    info!("Librrd version {}", librrd_version());
    rrd::ensure_rrd(&config);

    let (tx, _rx) = broadcast::channel(100);
    let shared_state = Arc::new(Mutex::new(AppState {
        wirkleistung: 0.0,
        zaehlerstand_diff: 0.0,
        einspeisung: 0.0,
        einspeisung_sts: false,
        tx,
    }));

    let mqtt_client = mqtt::init_mqtt(&config).await;

    // A) Serial Reader
    let state_serial = shared_state.clone();
    let config_serial = config.clone();
    let client_serial = mqtt_client.clone();
    tokio::spawn(async move {
        sml::run_serial_loop(config_serial, state_serial, client_serial, token).await;
    });

    // B) RRD Graph Generator
    let config_rrd = config.clone();
    tokio::spawn(async move {
        rrd::run_graph_loop(config_rrd, graph_token).await;
    });

    // C) Webserver
    let config_web = config.clone();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = web::start_server(
            &config_web.server_addr,
            &config_web.image_output_path,
            shared_state,
        )
        .await
        {
            error!("Webserver failed: {}", e);
        }
    });

    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");
    // Graceful Shutdown
    // Warte auf eines der beiden Signale
    tokio::select! {
        _ = sigterm.recv() => info!("SIGTERM received (systemd stop)"),
        _ = sigint.recv() => info!("SIGINT received (Ctrl+C)"),
    };

    info!("Shutting down application...");
    server_handle.abort();

    rrd::save_rrd_on_shutdown(&config);
    Ok(())
}
