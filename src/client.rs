use std::fmt::Debug;

use futures::{stream::SplitStream, Sink, SinkExt, StreamExt};
use gethostname::gethostname;
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    tungstenite::{handshake::client::generate_key, http::Request, Message},
    MaybeTlsStream, WebSocketStream,
};

use crate::{clipboard_handler, ClipboardSink, ClipboardSource};

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

impl<T> ClipboardSink for T
where
    T: Sink<Message> + Send + Unpin,
    <T as Sink<Message>>::Error: Debug,
{
    async fn publish(&mut self, data: String) -> anyhow::Result<()> {
        if data.is_empty() {
            self.send(Message::Ping(vec![]))
                .await
                .map_err(|e: <T as Sink<Message>>::Error| anyhow::anyhow!("{:?}", e))?;
        } else {
            self.send(Message::Text(data))
                .await
                .map_err(|e: <T as Sink<Message>>::Error| anyhow::anyhow!("{:?}", e))?;
        }
        Ok(())
    }
}
