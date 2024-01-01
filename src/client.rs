use std::fmt::Debug;

use futures::{stream::SplitStream, Sink, SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use gethostname::gethostname;
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    tungstenite::{handshake::client::generate_key, http::Request, Message},
    MaybeTlsStream, WebSocketStream,
};

use crate::{clipboard_handler, ClipboardContent, ClipboardSink, ClipboardSource};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ClientConfig {
    pub server_url: String,
    pub secret: Option<String>,
    pub client_id: Option<String>,
}

pub async fn clip_sync_svc(args: ClientConfig) -> anyhow::Result<()> {
    loop {
        if let Err(e) = clip_sync_svc_impl(args.clone()).await {
            log::error!("Error: {}", e);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

async fn clip_sync_svc_impl(args: ClientConfig) -> anyhow::Result<()> {
    let sender_id = args.client_id.unwrap_or(
        gethostname()
            .into_string()
            .unwrap_or(random_string::generate(12, "abcdefghijklmnopqrstuvwxyz")),
    );

    let url = url::Url::parse(&format!("{}/{}", &args.server_url, &sender_id))?;

    let req = Request::builder();
    let req = if args.secret.is_some() {
        req.header("Authorization", format!("Bearer {}", args.secret.unwrap()))
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

    let (write, read) = ws_stream.split();
    clipboard_handler::start(sender_id, read, write).await
}

impl ClipboardSource for SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    async fn poll(&mut self) -> anyhow::Result<String> {
        while let Some(msg) = self.next().await {
            match msg {
                Ok(Message::Text(text)) => return Ok(text),
                Ok(_) => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Err(anyhow::anyhow!("No message received"))
    }
}

struct WebSocketSink {
    sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    upload_url: String,
}

impl WebSocketSink {
    pub fn new(
        sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        device_id: &str,
        server_url: &str,
    ) -> anyhow::Result<Self> {
        let mut url = url::Url::parse(server_url).unwrap();
        if url.scheme() == "ws" {
            url.set_scheme("http").unwrap();
        } else if url.scheme() == "wss" {
            url.set_scheme("https").unwrap();
        } else {
            return Err(anyhow::anyhow!("Invalid scheme"));
        }
        url.set_path(&format!("/upload-image/{}", device_id));
        Ok(Self {
            sink,
            upload_url: url.into(),
        })
    }
}

impl ClipboardSink for WebSocketSink {
    async fn publish(&mut self, data: Option<ClipboardContent>) -> anyhow::Result<()> {
        let data = data.ok_or_else(|| anyhow::anyhow!("No data to publish"))?;
        let client = reqwest::Client::new();
        let res = client
            .post(&self.upload_url)
            .body(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        if !res.status().is_success() {
            return Err(anyhow::anyhow!("Upload failed"));
        }
        Ok(())
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
}

impl<T> ClipboardSink for T
where
    T: Sink<Message> + Send + Unpin,
    <T as Sink<Message>>::Error: Debug,
{
    async fn publish_raw_string(&mut self, data: Option<String>) -> anyhow::Result<()> {
        self.send(match data {
            Some(data) => Message::Text(data),
            None => Message::Ping(vec![]),
        })
        .await
        .map_err(|e: <T as Sink<Message>>::Error| anyhow::anyhow!("{:?}", e))?;
        Ok(())
    }
}
