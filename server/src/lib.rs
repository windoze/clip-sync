use futures_util::{SinkExt, StreamExt};
use log::{debug, warn};
use poem::{
    get, handler,
    listener::TcpListener,
    web::{
        websocket::{Message, WebSocket},
        Data, Html, Path,
    },
    EndpointExt, IntoResponse, Route, Server,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{channel, Receiver, Sender};

pub struct ServerArgs {
    pub endpoint: String,
    pub secret: Option<String>,
    pub use_tls: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

#[handler]
fn index() -> Html<&'static str> {
    // TODO: Add a proper index page
    Html("")
}

#[derive(Debug, Serialize, Deserialize)]
struct ClipboardData {
    source: String,
    data: String,
}

#[handler]
fn ws(Path(name): Path<String>, ws: WebSocket, sender: Data<&Sender<String>>) -> impl IntoResponse {
    let sender = sender.clone();
    let mut receiver = sender.subscribe();
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();

        let name_clone = name.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(data) = serde_json::from_str::<ClipboardData>(&text) {
                        debug!("{}: {}", data.source, data.data);
                        if name_clone != data.source {
                            warn!(
                                "Invalid message source '{}' from device '{name_clone}'.",
                                data.source
                            );
                            continue;
                        }
                    } else {
                        warn!("Invalid message: {} from device '{name_clone}'.", text);
                        continue;
                    }
                    if sender.send(text).is_err() {
                        warn!("Failed to publish message from device '{name_clone}'.");
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Ok(msg) = receiver.recv().await {
                if sink.send(Message::Text(msg)).await.is_err() {
                    warn!("Failed to send message to device '{}'.", &name);
                    break;
                }
            }
        });
    })
}

pub async fn message_stash(mut receiver: Receiver<String>) {
    // TODO: Store messages in a database
    while let Ok(msg) = receiver.recv().await {
        debug!("Message: {}", msg);
    }
}

pub async fn server_main() -> Result<(), std::io::Error> {
    let (sender, receiver) = channel::<String>(32);
    tokio::spawn(message_stash(receiver));
    let app = Route::new()
        .at("/", get(index))
        .at("/clip-sync/:device_id", get(ws.data(sender)));

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_server() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
        server_main().await.unwrap();
    }
}
