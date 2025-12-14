use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;
use tracing::{info, error};
use crate::config::Config;

pub async fn init_mqtt(config: &Config) -> AsyncClient {
    let mut mqttoptions = MqttOptions::new("sml1_rust", &config.mqtt_broker, config.mqtt_port);
    mqttoptions.set_credentials(&config.mqtt_user, &config.mqtt_pass);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(_) => {},
                Err(e) => {
                    error!("MQTT Connection Error: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });

    info!("MQTT connected to {}:{}", config.mqtt_broker, config.mqtt_port);
    send_mqtt_config(&client).await;
    client
}

async fn send_mqtt_config(client: &AsyncClient) {
    let configs = vec![
        ("homeassistant/sensor/sml/power/config", r#"{"unique_id": "sml.power","device_class": "power", "name": "Wirkleistung", "state_topic": "homeassistant/sensor/sml/wirkleistung/state","unit_of_measurement": "W"}"#),
        ("homeassistant/sensor/sml/bezug/config", r#"{"unique_id": "sml.bezug","state_class": "total_increasing", "device_class": "energy", "name": "Netzbezug", "state_topic": "homeassistant/sensor/sml/zaehler/state","unit_of_measurement": "Wh", "value_template": "{{ value_json.bezug}}"}"#),
        ("homeassistant/sensor/sml/einspeisung/config", r#"{"unique_id": "sml.einspeisung","state_class": "total_increasing", "device_class": "energy", "name": "Netzeinspeisung", "state_topic": "homeassistant/sensor/sml/zaehler/state","unit_of_measurement": "Wh", "value_template": "{{ value_json.einspeisung}}"}"#),
        ("homeassistant/binary_sensor/sml/feed/config", r#"{"unique_id": "sml.feed", "device_class": "power", "name": "Einspeisung", "state_topic": "homeassistant/binary_sensor/sml/feed/state"}"#)
    ];

    for (topic, payload) in configs {
        let _ = client.publish(topic, QoS::AtLeastOnce, true, payload).await;
    }
}