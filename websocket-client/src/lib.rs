use std::fmt::Debug;

use futures::{stream::SplitStream, SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use gethostname::gethostname;
use log::{debug, info, warn};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    tungstenite::{handshake::client::generate_key, http::Request, Message},
    MaybeTlsStream, WebSocketStream,
};

use client_interface::{
    ClipSyncClient, ClipboardContent, ClipboardRecord, ClipboardSink, ClipboardSource,
    ServerClipboardContent, ServerClipboardRecord,
};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ClientConfig {
    pub server_url: String,
    pub secret: Option<String>,
    pub client_id: Option<String>,
}

pub struct WebsocketClipSyncClient;

impl ClipSyncClient for WebsocketClipSyncClient {
    type Config = ClientConfig;

    #[allow(refining_impl_trait)]
    async fn connect(
        args: Self::Config,
    ) -> anyhow::Result<(String, WebSocketSource, WebSocketSink)> {
        let sender_id = args.client_id.unwrap_or(
            gethostname()
                .into_string()
                .unwrap_or(random_string::generate(12, "abcdefghijklmnopqrstuvwxyz")),
        );

        let server_url = if args.server_url.ends_with('/') {
            format!("{}api/clip-sync/{}", &args.server_url, &sender_id)
        } else {
            format!("{}/api/clip-sync/{}", &args.server_url, &sender_id)
        };

        let mut url = url::Url::parse(&server_url)?;
        if url.scheme() == "http" || url.scheme() == "ws" {
            url.set_scheme("ws").unwrap();
        } else {
            url.set_scheme("wss").unwrap();
        }
        info!("Connecting to {} ...", url);

        let req = Request::builder();
        let req = if args.secret.is_some() {
            req.header(
                "Authorization",
                format!("Bearer {}", args.secret.clone().unwrap()),
            )
        } else {
            req
        }
        .method("GET")
        .header("Host", url.host_str().unwrap_or_default())
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", generate_key())
        .uri(url.as_str())
        .body(())?;

        let (ws_stream, _) = tokio_tungstenite::connect_async(req).await?;
        info!("Connected to {}", url);

        let (write, read) = ws_stream.split();
        let write = WebSocketSink::new(write, &sender_id, &args.server_url, args.secret.clone())?;
        let read = WebSocketSource::new(read, &args.server_url, args.secret.clone())?;
        Ok((sender_id, read, write))
    }
}

pub struct WebSocketSource {
    source: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    image_url: String,
    secret: Option<String>,
}

impl WebSocketSource {
    pub fn new(
        source: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        server_url: &str,
        secret: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut url = url::Url::parse(server_url).unwrap();
        if url.scheme() == "ws" || url.scheme() == "http" {
            url.set_scheme("http").unwrap();
        } else if url.scheme() == "wss" || url.scheme() == "https" {
            url.set_scheme("https").unwrap();
        } else {
            return Err(anyhow::anyhow!("Invalid scheme"));
        }
        url.set_path("/api/images");
        Ok(Self {
            source,
            image_url: url.into(),
            secret,
        })
    }

    async fn poll_raw_string(&mut self) -> anyhow::Result<Option<String>> {
        while let Some(msg) = self.source.next().await {
            match msg {
                Ok(Message::Text(text)) => return Ok(Some(text)),
                Ok(_) => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(None)
    }

    async fn download_image(&mut self, url: &str) -> anyhow::Result<client_interface::ImageData> {
        debug!("Downloading image from {}", url);
        let url = format!("{}/{}", self.image_url, url);
        let client = reqwest::Client::new();
        let req = client.get(&url);
        let req = match &self.secret {
            Some(secret) => req.bearer_auth(secret),
            None => req,
        };
        let res = req.send().await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        if !res.status().is_success() {
            warn!("Download failed: {}", res.status());
            return Err(anyhow::anyhow!("Download failed"));
        }
        info!("Image downloaded from {}", url);
        client_interface::ImageData::from_png(&res.bytes().await?)
    }
}

impl ClipboardSource for WebSocketSource {
    async fn poll(&mut self) -> anyhow::Result<ClipboardRecord> {
        let raw_string = self.poll_raw_string().await?;
        debug!("+++Received message: {:?}", raw_string);
        if let Some(raw_string) = raw_string {
            let data: ServerClipboardRecord = serde_json::from_str(&raw_string)?;
            match data.content {
                ServerClipboardContent::Text(text) => Ok(ClipboardRecord {
                    source: data.source,
                    content: ClipboardContent::Text(text),
                }),
                ServerClipboardContent::ImageUrl(url) => Ok(ClipboardRecord {
                    source: data.source,
                    content: ClipboardContent::Image(self.download_image(&url).await?),
                }),
            }
        } else {
            Err(anyhow::anyhow!("No message received"))
        }
    }
}

pub struct WebSocketSink {
    sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    upload_url: String,
    secret: Option<String>,
}

impl WebSocketSink {
    pub fn new(
        sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        device_id: &str,
        server_url: &str,
        secret: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut url = url::Url::parse(server_url).unwrap();
        if url.scheme() == "ws" || url.scheme() == "http" {
            url.set_scheme("http").unwrap();
        } else if url.scheme() == "wss" || url.scheme() == "https" {
            url.set_scheme("https").unwrap();
        } else {
            return Err(anyhow::anyhow!("Invalid scheme"));
        }
        url.set_path(&format!("/api/upload-image/{}", device_id));
        Ok(Self {
            sink,
            upload_url: url.into(),
            secret,
        })
    }

    async fn publish_raw_string(&mut self, data: Option<String>) -> anyhow::Result<()> {
        self.sink
            .send(match data {
                Some(data) => Message::Text(data),
                None => Message::Ping(vec![]),
            })
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        Ok(())
    }

    async fn upload_image(&mut self, data: &client_interface::ImageData) -> anyhow::Result<String> {
        debug!("Uploading image to {}", self.upload_url);
        let client = reqwest::Client::new();
        let part = reqwest::multipart::Part::bytes(data.to_png()?).mime_str("image/png")?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let req = client.post(&self.upload_url);
        let req = match &self.secret {
            Some(secret) => req.bearer_auth(secret),
            None => req,
        };
        let res = req
            .multipart(form)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        if !res.status().is_success() {
            warn!("Upload failed: {}", res.status());
            return Err(anyhow::anyhow!("Upload failed"));
        }
        let image_url = res.text().await?;
        debug!("Image uploaded to {}", image_url);
        Ok(image_url)
    }
}

impl ClipboardSink for WebSocketSink {
    async fn publish(&mut self, data: Option<ClipboardRecord>) -> anyhow::Result<()> {
        let raw_string = match data {
            Some(data) => match data.content {
                ClipboardContent::Text(text) => {
                    let data = ServerClipboardRecord {
                        source: data.source,
                        content: ServerClipboardContent::Text(text),
                    };
                    Some(serde_json::to_string(&data)?)
                }
                ClipboardContent::Image(img) => {
                    // Convert data to ServerClipboardData
                    let data = ServerClipboardRecord {
                        source: data.source,
                        content: ServerClipboardContent::ImageUrl(self.upload_image(&img).await?),
                    };
                    Some(serde_json::to_string(&data)?)
                }
            },
            None => None,
        };
        self.publish_raw_string(raw_string).await?;
        Ok(())
    }
}
