use gethostname::gethostname;
use log::{debug, warn};
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use serde::Deserialize;

use client_interface::{ClipSyncClient, ClipboardRecord, ClipboardSink, ClipboardSource};

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

pub struct MqttSubscriber {
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
                    let data: ClipboardRecord = bincode::deserialize(&p.payload)?;
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

pub struct MqttPublisher {
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
        if let Some(data) = data.map(|d| bincode::serialize(&d).unwrap()) {
            self.client
                .publish(self.topic.clone(), QoS::AtLeastOnce, false, data)
                .await
                .ok();
        }
        Ok(())
    }
}

pub struct MqttClipSyncClient;

impl ClipSyncClient for MqttClipSyncClient {
    type Config = MqttClientConfig;

    #[allow(refining_impl_trait)]
    async fn connect(
        args: Self::Config,
    ) -> anyhow::Result<(String, MqttSubscriber, MqttPublisher)> {
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
        options.set_max_packet_size(1024 * 1024 * 100, 1024 * 1024 * 100); // 100 MB

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
        Ok((sender_id, source, sink))
    }
}
