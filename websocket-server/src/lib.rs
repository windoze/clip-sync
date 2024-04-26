use std::{path::PathBuf, sync::Arc, time::Duration};

use chrono::Utc;
use client_interface::{ClipboardMessage, QueryParams, QueryResult, ServerClipboardContent};
use futures_util::{SinkExt, StreamExt};
use log::{debug, info, trace, warn};
use poem::{
    endpoint::StaticFilesEndpoint,
    get, handler,
    http::StatusCode,
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
    middleware::Cors,
    post,
    web::{
        websocket::{Message, WebSocket},
        Data, Json, Multipart, Path,
    },
    EndpointExt, IntoResponse, Request, Route, Server,
};
use serde::Deserialize;
use sha2::Digest;
use tokio::sync::{broadcast::channel, RwLock};

use crate::global_state::GlobalState;

mod auth;
mod global_state;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ServerConfig {
    pub endpoint: String,
    pub secret: Option<String>,
    #[serde(default)]
    pub use_tls: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub web_root: Option<PathBuf>,
    pub index_path: Option<String>,
    pub image_path: Option<PathBuf>,
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
                    trace!("Pong from device '{}'.", &name_clone);
                    continue;
                }
                if let Message::Text(text) = msg {
                    if let Ok(data) = serde_json::from_str::<ClipboardMessage>(&text) {
                        if name_clone != data.entry.source {
                            warn!(
                                "Invalid message source '{}' from device '{name_clone}'.",
                                data.entry.source
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
                        if msg.entry.source == name {
                            continue;
                        }
                        if sink
                            .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                            .await
                            .is_err()
                        {
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
                        trace!("Sending ping to device '{}'.", &name);
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
    let mut device_list = global_state.read().await.get_device_list();
    device_list.sort();
    Json(device_list)
}

#[handler]
async fn get_online_device_list(data: Data<&Arc<RwLock<GlobalState>>>) -> impl IntoResponse {
    let global_state = data.0.clone();
    let mut device_list = global_state.read().await.get_online_device_list();
    device_list.sort();
    Json(device_list)
}

#[allow(clippy::too_many_arguments)]
#[handler]
async fn query(
    req: &Request,
    data: Data<&Arc<RwLock<GlobalState>>>,
) -> poem::Result<Json<QueryResult>> {
    let params = req.params::<QueryParams>()?;
    debug!("Query: {:?}", params);
    let global_state = data.0.clone();

    // Json(serde_json::to_string(&device_list).unwrap())
    let ret = global_state.read().await.query(params).await;
    match ret {
        Ok(entries) => Ok(Json(entries)),
        Err(e) => {
            warn!("Failed to query: {}", e);
            Err(poem::Error::from_status(StatusCode::BAD_REQUEST))
        }
    }
}

#[handler]
async fn upload_image(
    Path(name): Path<String>,
    mut multipart: Multipart,
    data: Data<&Arc<RwLock<GlobalState>>>,
) -> poem::Result<String> {
    if let Ok(Some(field)) = multipart.next_field().await {
        if field.content_type().unwrap_or("") != "image/png" {
            return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
        }
        let timestamp = Utc::now().format("%Y-%m-%d-%H-%M-%S-%6f");
        let dir: PathBuf = data.0.read().await.get_image_path().join(&name);
        tokio::fs::create_dir_all(&dir).await.map_err(|e| {
            warn!("Failed to create directory: {}", e);
            poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
        let mut suffix = 1usize;
        loop {
            let filename = format!("{}-{}.png", timestamp, suffix);
            let path: PathBuf = dir.join(&filename);
            if path.exists() {
                suffix += 1;
                continue;
            }
            break;
        }
        let filename: String = format!("{}-{}.png", timestamp, suffix);
        let filepath: PathBuf = dir.join(&filename);
        let part_name = field.name().map(ToString::to_string);
        let file_name = field.file_name().map(ToString::to_string);
        if let Ok(bytes) = field.bytes().await {
            println!(
                "name={:?} filename={:?} length={}, save={:?}",
                part_name,
                file_name,
                bytes.len(),
                filepath,
            );
            let mut hasher = <sha2::Sha512 as Digest>::new();
            hasher.update(&bytes);
            let digest = hex::encode(Into::<[u8; 64]>::into(hasher.finalize()));
            let existing_entry = data.0.read().await.get_entry_by_id(&digest).await.unwrap();
            if let Some(existing_entry) = existing_entry {
                if let ServerClipboardContent::ImageUrl(url) = &existing_entry.entry.content {
                    debug!("Image already exists: {:?}", existing_entry);
                    return Ok(url.clone());
                }
            }
            tokio::fs::write(&filepath, bytes).await.map_err(|e| {
                warn!("Failed to write file: {}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;
            debug!("Image saved to {:?}", filepath);
        }
        return Ok(format!("{name}/{}", filename));
    }
    warn!("No image data received.");
    Err(poem::Error::from_status(StatusCode::BAD_REQUEST))
}

#[handler]
async fn get_image_collection(
    Path(name): Path<String>,
    data: Data<&Arc<RwLock<GlobalState>>>,
) -> poem::Result<Json<Vec<String>>> {
    let dir: PathBuf = data.0.read().await.get_image_path().join(&name);
    let mut ret = Vec::new();
    let mut entries = tokio::fs::read_dir(&dir).await.map_err(|e| {
        warn!("Failed to read directory: {}", e);
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        info!("Entry: {:?}", entry);
        if let Some(filename) = entry.file_name().to_str() {
            ret.push(format!("{}/{}", name, filename));
        }
    }
    ret.sort();
    Ok(Json(ret))
}

fn api(
    args: ServerConfig,
    global_state: Arc<RwLock<GlobalState>>,
) -> auth::ApiKeyAuthEndpoint<poem::middleware::CorsEndpoint<poem::Route>> {
    let image_path = args.image_path.clone().unwrap();
    Route::new()
        .at("/clip-sync/:device_id", get(ws.data(global_state.clone())))
        .at(
            "/device-list",
            get(get_device_list).data(global_state.clone()),
        )
        .at(
            "/online-device-list",
            get(get_online_device_list).data(global_state.clone()),
        )
        .at("/query", get(query).data(global_state.clone()))
        .at(
            "/collection/:device_id",
            get(get_image_collection).data(global_state.clone()),
        )
        .nest(
            "/images",
            StaticFilesEndpoint::new(image_path).show_files_listing(),
        )
        .at(
            "/upload-image/:device_id",
            post(upload_image.data(global_state.clone())),
        )
        .with(Cors::new())
        .with(auth::ApiKeyAuth::new(args.secret))
}

pub async fn server_main(mut args: ServerConfig) -> Result<(), std::io::Error> {
    let (sender, _) = channel::<ClipboardMessage>(32);
    if args.image_path.is_none() {
        args.image_path = Some(PathBuf::from("./images"));
    }
    if args.web_root.is_none() {
        args.web_root = Some(PathBuf::from("./static-files"));
    }
    let global_state = Arc::new(RwLock::new(GlobalState::new(&args, sender)));
    let app = Route::new()
        .nest(
            "/",
            StaticFilesEndpoint::new(args.web_root.as_ref().unwrap()).index_file("index.html"),
        )
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_serde() {
        use client_interface::{ServerClipboardContent, ServerClipboardRecord};
        let data = ServerClipboardRecord {
            id: None,
            source: "test".to_string(),
            content: ServerClipboardContent::Text("test".to_string()),
        };
        let json = serde_json::to_string(&data).unwrap();
        println!("{}", json);
        let data2: ServerClipboardRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(data, data2);

        let msg = super::ClipboardMessage {
            entry: data,
            timestamp: 0,
        };
        let json = serde_json::to_string(&msg).unwrap();
        println!("{}", json);
    }
}
