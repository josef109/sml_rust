use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(long, env, default_value = "/dev/ttyUSB0")]
    pub serial_port: String,

    #[arg(long, env, default_value = "localhost")]
    pub mqtt_broker: String,

    #[arg(long, env, default_value_t = 1883)]
    pub mqtt_port: u16,

    #[arg(long, env)]
    pub mqtt_user: String,

    #[arg(long, env)]
    pub mqtt_pass: String,

    #[arg(long, env, default_value = "/tmp/sml_rust/ehz.rrd")]
    pub rrd_path: PathBuf,

    #[arg(long, env, default_value = "./bak/ehz.rrd")]
    pub rrd_backup_path: String,

    #[arg(long, env, default_value = "/tmp/sml_rust")]
    pub image_output_path: String,

    #[arg(long, env, default_value = "0.0.0.0:5000")]
    pub server_addr: String,
}
