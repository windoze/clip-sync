use futures_util::{SinkExt, StreamExt};
use log::{debug, warn};
use poem::{
    get, handler,
    http::StatusCode,
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
    web::{
        headers::{self, authorization::Bearer, HeaderMapExt},
        websocket::{Message, WebSocket},
        Data, Html, Path,
    },
    Endpoint, EndpointExt, IntoResponse, Middleware, Request, Route, Server,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{channel, Receiver, Sender};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ServerConfig {
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

struct ApiKeyAuth {
    api_key: Option<String>,
}

impl<E: Endpoint> Middleware<E> for ApiKeyAuth {
    type Output = ApiKeyAuthEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ApiKeyAuthEndpoint {
            ep,
            api_key: self.api_key.clone(),
        }
    }
}

struct ApiKeyAuthEndpoint<E> {
    ep: E,
    api_key: Option<String>,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for ApiKeyAuthEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        // Skip auth if no api key is set
        if self.api_key.is_none() {
            return self.ep.call(req).await;
        }
        if let Some(auth) = req.headers().typed_get::<headers::Authorization<Bearer>>() {
            if auth.0.token() == self.api_key.as_ref().unwrap() {
                return self.ep.call(req).await;
            }
        }
        Err(poem::Error::from_status(StatusCode::UNAUTHORIZED))
    }
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

pub async fn server_main(args: ServerConfig) -> Result<(), std::io::Error> {
    let (sender, receiver) = channel::<String>(32);
    tokio::spawn(message_stash(receiver));
    let app = Route::new()
        .at("/", get(index))
        .at("/clip-sync/:device_id", get(ws.data(sender)))
        .with(ApiKeyAuth {
            api_key: args.secret,
        });

    let listener = TcpListener::bind(args.endpoint);
    if args.use_tls {
        let cert = std::fs::read(args.cert_path.unwrap())?;
        let key = std::fs::read(args.key_path.unwrap())?;
        Server::new(
            listener
                .rustls(RustlsConfig::new().fallback(RustlsCertificate::new().key(key).cert(cert))),
        )
        .run(app)
        .await?;
    } else {
        Server::new(listener).run(app).await?;
    }
    Ok(())
}
