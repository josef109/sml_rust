| Argument | Env Variable | Default | Description |
| -------- | ------------ | ------- | ----------- |
--serial-port|SERIAL_PORT|/dev/ttyUSB0|Path to the USB IR reader.
--mqtt-broker|MQTT_BROKER|localhost|IP/Hostname of MQTT Broker.
--mqtt-port|MQTT_PORT|1883|MQTT Port.
--mqtt-user|MQTT_USER|(Empty)|MQTT Username.
--mqtt-pass|MQTT_PASS|(Empty)|MQTT Password.
--rrd-path|RRD_PATH|/tmp/sml_rust/ehz.rrd|Path to the active RRD database.
--rrd-backup-path|RRD_BACKUP_PATH|./bak/ehz.rrd|Path for shutdown backups.
--image-output-path|IMAGE_OUTPUT_PATH|/tmp/sml_rust|Directory for generated PNG graphs.
--server-addr|SERVER_ADDR|0.0.0.0:5000|Web server bind address.

---
## Example Usage
#### Run with custom serial port and MQTT broker
cargo run --release -- --serial-port /dev/ttyUSB0 --mqtt-broker 192.168.1.10