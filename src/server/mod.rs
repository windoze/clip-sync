use std::{ops::Bound, path::PathBuf, sync::Arc, time::Duration};

use chrono::{TimeZone, Utc};
use futures_util::{SinkExt, StreamExt};
use log::{debug, info, warn};
use poem::{
    endpoint::StaticFilesEndpoint,
    error::StaticFileError,
    get, handler,
    http::StatusCode,
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
    middleware::Cors,
    web::{
        headers::{HeaderMapExt, Range},
        websocket::{Message, WebSocket},
        Data, Html, Json, Path, StaticFileResponse,
    },
    Body, EndpointExt, IntoResponse, Request, Route, Server,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast::channel, RwLock};

use crate::{server::global_state::GlobalState, APP_ICON};

mod auth;
mod global_state;
mod search;

pub use search::QueryParam;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ServerConfig {
    pub endpoint: String,
    pub secret: Option<String>,
    #[serde(default)]
    pub use_tls: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub index_path: Option<PathBuf>,
}

#[handler]
fn index() -> Html<&'static str> {
    // TODO: Add a proper index page
    Html("<html><head><title>ClipSync</title></head><body>ClipSync Server</body></html>")
}

#[handler]
fn fav_icon(req: &Request) -> Result<StaticFileResponse, StaticFileError> {
    let range = req.headers().typed_get::<Range>();
    let mut content_range = None;
    let mut content_length;
    let body = if let Some((start, end)) = range.and_then(|range| range.iter().next()) {
        let start = match start {
            Bound::Included(n) => n,
            Bound::Excluded(n) => n + 1,
            Bound::Unbounded => 0,
        };
        let end = match end {
            Bound::Included(n) => n + 1,
            Bound::Excluded(n) => n,
            Bound::Unbounded => APP_ICON.len() as u64,
        };
        if end < start || end > APP_ICON.len() as u64 {
            // builder.typed_header(ContentRange::unsatisfied_bytes(length))
            return Err(StaticFileError::RangeNotSatisfiable {
                size: APP_ICON.len() as u64,
            });
        }

        if start != 0 || end != APP_ICON.len() as u64 {
            content_range = Some((start..end, APP_ICON.len() as u64));
        }
        content_length = end - start;
        Body::from(&APP_ICON[start as usize..end as usize])
    } else {
        content_length = APP_ICON.len() as u64;
        Body::from(APP_ICON)
    };
    Ok(StaticFileResponse::Ok {
        body,
        content_length,
        content_type: Some("image/png".to_string()),
        etag: None,
        last_modified: None,
        content_range,
    })
}

fn default_timestamp() -> i64 {
    Utc::now().timestamp()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ClipboardData {
    source: String,
    data: String,
    #[serde(default = "default_timestamp")]
    timestamp: i64,
}

#[handler]
async fn ws(
    Path(name): Path<String>,
    ws: WebSocket,
    data: Data<&Arc<RwLock<GlobalState>>>,
) -> impl IntoResponse {
    debug!("New connection from device '{}'.", &name);
    let global_state = data.0.clone();
    let mut receiver = global_state.read().await.get_receiver();
    ws.on_upgrade(move |socket| async move {
        info!("Websocket to device '{}' created.", &name);
        let (mut sink, mut stream) = socket.split();
        global_state.write().await.add_device(&name);

        let name_clone = name.clone();
        let global_state_clone = global_state.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Pong(_) = msg {
                    debug!("Pong from device '{}'.", &name_clone);
                    continue;
                }
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
                        if global_state_clone
                            .read()
                            .await
                            .add_entry(data, true)
                            .await
                            .is_err()
                        {
                            warn!("Failed to add entry from device '{name_clone}'.");
                            break;
                        }
                    } else {
                        warn!("Invalid message: {} from device '{name_clone}'.", text);
                        continue;
                    }
                }
            }
            global_state_clone.write().await.remove_device(&name_clone);
        });

        tokio::spawn(async move {
            loop {
                match tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await {
                    Ok(Ok(msg)) => {
                        if sink.send(Message::Text(msg)).await.is_err() {
                            warn!("Failed to send message to device '{}'.", &name);
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        // Channel closed
                        warn!("Channel closed: {}", e);
                        break;
                    }
                    Err(_) => {
                        // Timeout
                        debug!("Sending ping to device '{}'.", &name);
                        match sink.send(Message::Ping(vec![])).await {
                            Ok(_) => continue,
                            Err(e) => {
                                warn!("Failed to send ping: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            global_state.write().await.remove_device(&name);
        });
    })
}

#[handler]
async fn get_device_list(data: Data<&Arc<RwLock<GlobalState>>>) -> impl IntoResponse {
    let global_state = data.0.clone();
    let device_list = global_state.read().await.get_device_list();
    Json(device_list)
}

#[handler]
async fn get_online_device_list(data: Data<&Arc<RwLock<GlobalState>>>) -> impl IntoResponse {
    let global_state = data.0.clone();
    let device_list = global_state.read().await.get_online_device_list();
    Json(device_list)
}

#[derive(Debug, Clone, Deserialize)]
struct Params {
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    begin: Option<i64>,
    #[serde(default)]
    end: Option<i64>,
    #[serde(default)]
    size: Option<usize>,
    #[serde(default)]
    skip: Option<usize>,
    #[serde(default)]
    sort: Option<String>,
}

impl From<Params> for QueryParam {
    fn from(val: Params) -> Self {
        QueryParam {
            query: val.q,
            sources: val
                .from
                .unwrap_or_default()
                .split(',')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            time_range: match (val.begin, val.end) {
                (Some(begin), Some(end)) => Some((
                    Utc.timestamp_opt(begin, 0).unwrap(),
                    Utc.timestamp_opt(end, 0).unwrap(),
                )),
                (Some(begin), None) => Some((Utc.timestamp_opt(begin, 0).unwrap(), Utc::now())),
                (None, Some(end)) => Some((
                    Utc.timestamp_opt(0, 0).unwrap(),
                    Utc.timestamp_opt(end, 0).unwrap(),
                )),
                _ => None,
            },
            skip: val.skip.unwrap_or(0),
            size: val.size.unwrap_or(10),
            sort_by_score: val.sort.unwrap_or("time".to_string()) == "score",
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[handler]
async fn query(
    req: &Request,
    data: Data<&Arc<RwLock<GlobalState>>>,
) -> poem::Result<Json<Vec<ClipboardData>>> {
    let params = req.params::<Params>()?;
    debug!("Query: {:?}", params);
    let global_state = data.0.clone();
    let param = params.into();

    // Json(serde_json::to_string(&device_list).unwrap())
    let ret = global_state.read().await.query(param).await;
    match ret {
        Ok(entries) => Ok(Json(entries)),
        Err(e) => {
            warn!("Failed to query: {}", e);
            Err(poem::Error::from_status(StatusCode::BAD_REQUEST))
        }
    }
}

fn api(
    args: ServerConfig,
    global_state: Arc<RwLock<GlobalState>>,
) -> auth::ApiKeyAuthEndpoint<poem::middleware::CorsEndpoint<poem::Route>> {
    Route::new()
        .at(
            "/device-list",
            get(get_device_list).data(global_state.clone()),
        )
        .at(
            "/online-device-list",
            get(get_online_device_list).data(global_state.clone()),
        )
        .at("/query", get(query).data(global_state.clone()))
        .with(Cors::new())
        .with(auth::ApiKeyAuth::new(args.secret))
}

pub async fn server_main(args: ServerConfig) -> Result<(), std::io::Error> {
    let (sender, _) = channel::<String>(32);
    let global_state = Arc::new(RwLock::new(GlobalState::new(&args, sender)));
    let app = Route::new()
        .nest(
            "/",
            StaticFilesEndpoint::new("./clip-sync-ui/dist").index_file("index.html"),
        )
        .at("/favicon.ico", get(fav_icon))
        .at("/clip-sync/:device_id", get(ws.data(global_state.clone())))
        .nest("/api", api(args.clone(), global_state));

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
