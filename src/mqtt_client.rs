use gethostname::gethostname;
use log::{debug, warn};
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use serde::Deserialize;

use crate::{clipboard_handler, ClipboardRecord, ClipboardSink, ClipboardSource};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct MqttClientConfig {
    pub mqtt_server_addr: String,
    pub mqtt_server_port: u16,
    pub mqtt_topic: Option<String>,
    pub mqtt_username: Option<String>,
    pub mqtt_password: Option<String>,
    pub mqtt_client_id: Option<String>,
}

struct MqttSubscriber {
    eventloop: EventLoop,
    device_id: String,
}

impl MqttSubscriber {
    pub fn new(eventloop: EventLoop, device_id: String) -> Self {
        Self {
            eventloop,
            device_id,
        }
    }
}

impl ClipboardSource for MqttSubscriber {
    async fn poll(&mut self) -> anyhow::Result<ClipboardRecord> {
        loop {
            match self.eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(p))) => {
                    let data: ClipboardRecord = serde_json::from_slice(&p.payload)?;
                    if data.source == self.device_id {
                        debug!("Skipping clipboard update from self");
                        continue;
                    }
                    return Ok(data);
                }
                Ok(_) => {
                    // Other events are ignored
                    continue;
                }
                Err(e) => {
                    warn!("Error: {}", e);
                    return Err(e)?;
                }
            }
        }
    }
}

struct MqttPublisher {
    client: AsyncClient,
    topic: String,
}

impl MqttPublisher {
    pub fn new(client: AsyncClient, topic: String) -> Self {
        Self { client, topic }
    }
}

impl ClipboardSink for MqttPublisher {
    async fn publish(&mut self, data: Option<ClipboardRecord>) -> anyhow::Result<()> {
        if let Some(data) = data.map(|d| serde_json::to_string(&d).unwrap()) {
            self.client
                .publish(self.topic.clone(), QoS::AtLeastOnce, false, data)
                .await
                .ok();
        }
        Ok(())
    }
}

pub async fn clip_sync_svc(args: MqttClientConfig) -> anyhow::Result<()> {
    let sender_id = args.mqtt_client_id.unwrap_or(
        gethostname()
            .into_string()
            .unwrap_or(random_string::generate(12, "abcdefghijklmnopqrstuvwxyz")),
    );
    let mut options = MqttOptions::new(
        sender_id.clone(),
        args.mqtt_server_addr,
        args.mqtt_server_port,
    );

    if args.mqtt_username.is_some() || args.mqtt_password.is_some() {
        options.set_credentials(
            args.mqtt_username.unwrap_or_default(),
            args.mqtt_password.unwrap_or_default(),
        );
    }

    let topic = args.mqtt_topic.unwrap_or("clipboard".to_string());
    let (client, eventloop) = AsyncClient::new(options, 10);
    client.subscribe(topic.clone(), QoS::AtLeastOnce).await?;

    let sink = MqttPublisher::new(client.clone(), topic.clone());
    let source = MqttSubscriber::new(eventloop, sender_id.clone());
    clipboard_handler::start(sender_id, source, sink).await
}
