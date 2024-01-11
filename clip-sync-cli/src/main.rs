use std::{path::PathBuf, sync::Arc};

use clap::{Parser, Subcommand};

use client_interface::{ClipSyncClient, ClipboardMessage, ClipboardRecord, ImageData};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

mod client;

#[derive(Debug, Subcommand)]
enum Commands {
    #[cfg(feature = "websocket")]
    #[command(aliases = &["l"])]
    ListDevices {
        #[arg(short, long, default_value = "false")]
        online_only: bool,
    },
    #[cfg(feature = "websocket")]
    #[command(aliases = &["s"])]
    Search {
        #[clap(index = 1)]
        text: Option<String>,
        #[arg(short, long)]
        skip: Option<usize>,
        #[arg(short, long)]
        limit: Option<usize>,
        #[arg(short, long)]
        device: Vec<String>,
    },
    /// Send text to the server
    #[command(arg_required_else_help = true, aliases = &["text", "t"])]
    SendText {
        /// Text to send, or path to file to send if prefixed with '@'
        #[clap(index = 1)]
        text_or_file: String,
    },
    /// Send image to the server
    #[command(arg_required_else_help = true, aliases = &["image", "i"])]
    SendImage {
        /// Path to image file to send, omit to read from stdin
        #[clap(index = 1)]
        path: Option<PathBuf>,
    },
    /// Monitor clipboard content
    #[command(aliases = &["mon", "m"])]
    Monitor {
        /// Path to file to write clipboard content to, omit to write to stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Directory to write images to, omit to ignore images
        #[arg(short, long)]
        image_dir: Option<PathBuf>,
        /// Whether to include timestamp in output
        #[arg(short, long, default_value = "true")]
        timestamp: bool,
        /// Whether to include source in output
        #[arg(short, long, default_value = "true")]
        source: bool,
        /// Whether to escape special characters
        #[arg(short, long, default_value = "false")]
        escape: bool,
    },
}

#[derive(Debug, Parser)] // requires `derive` feature
struct Cli {
    /// Path to config file
    #[arg(short, long = "config")]
    config_path: Option<std::path::PathBuf>,
    /// Whether to output JSON
    #[arg(short, long, default_value = "false")]
    json: bool,
    /// Verbosity level
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    /// Subcommand
    #[command(subcommand)]
    command: Commands,
}

async fn start_msg_client(
    args: &clip_sync_config::Args,
) -> anyhow::Result<(
    String,
    tokio::sync::mpsc::Sender<Option<ClipboardRecord>>,
    tokio::sync::mpsc::Receiver<ClipboardRecord>,
    tokio::task::JoinHandle<()>,
)> {
    let mut args = args.clone();
    let (sender, clipboard_publisher_receiver) = tokio::sync::mpsc::channel(10);
    let (clipboard_subscriber_sender, receiver) = tokio::sync::mpsc::channel(10);

    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let (client_id, join_handler) = if args.roles.contains(&("websocket-client").to_string()) {
        args.websocket_client.client_id = Some(
            args.websocket_client
                .client_id
                .unwrap_or("$cli".to_string()),
        );
        let (client_id, source, sink) =
            websocket_client::WebsocketClipSyncClient::connect(args.websocket_client).await?;
        let client_id_clone = client_id.clone();
        (
            client_id,
            tokio::spawn(async move {
                let publisher_task = client::clipboard_publisher(
                    sink,
                    clipboard_publisher_receiver,
                    shutdown.clone(),
                );
                let subscriber_task = client::clipboard_subscriber(
                    source,
                    clipboard_subscriber_sender,
                    client_id_clone,
                    shutdown.clone(),
                );

                let (r1, r2) = tokio::join!(publisher_task, subscriber_task);
                r1.ok();
                r2.ok();
            }),
        )
    } else {
        #[cfg(feature = "mqtt")]
        if args.roles.contains(&("mqtt-client").to_string()) {
            args.mqtt_client.mqtt_client_id = Some(
                args.mqtt_client
                    .mqtt_client_id
                    .unwrap_or("$cli".to_string()),
            );
            let (client_id, source, sink) =
                mqtt_client::MqttClipSyncClient::connect(args.mqtt_client).await?;
            let client_id_clone = client_id.clone();
            (
                client_id,
                tokio::spawn(async move {
                    let publisher_task = client::clipboard_publisher(
                        sink,
                        clipboard_publisher_receiver,
                        shutdown.clone(),
                    );
                    let subscriber_task = client::clipboard_subscriber(
                        source,
                        clipboard_subscriber_sender,
                        client_id_clone,
                        shutdown.clone(),
                    );

                    let (r1, r2) = tokio::join!(publisher_task, subscriber_task);
                    r1.ok();
                    r2.ok();
                }),
            )
        } else {
            panic!("No client role specified");
        }
    };
    Ok((client_id, sender, receiver, join_handler))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .filter_module("tantivy", log::LevelFilter::Warn) // Tantivy is too talky at the INFO level
        .init();
    let args = clip_sync_config::parse_config(cli.config_path)?;

    match cli.command {
        #[cfg(feature = "websocket")]
        Commands::ListDevices { online_only } => {
            if let Some(url) = args.get_server_url() {
                let url = if online_only {
                    format!("{}api/online-device-list", url)
                } else {
                    format!("{}api/device-list", url)
                };
                let client = reqwest::Client::new();
                let resp: Vec<String> = client.get(&url).send().await?.json().await?;
                let devices: Vec<String> =
                    resp.into_iter().filter(|d| !d.starts_with('$')).collect();
                if cli.json {
                    println!("{}", serde_json::to_string(&devices)?);
                } else {
                    for device in devices {
                        println!("{}", device);
                    }
                }
            }
        }
        #[cfg(feature = "websocket")]
        Commands::Search {
            text,
            skip,
            limit,
            device,
        } => {
            if let Some(url) = args.get_server_url() {
                let url = format!("{}api/query", url);
                let client = reqwest::Client::new();
                let param = client_interface::Params {
                    q: text,
                    from: if device.is_empty() {
                        None
                    } else {
                        Some(device.join(","))
                    },
                    begin: None,
                    end: None,
                    size: limit,
                    skip,
                }
                .to_query();
                #[allow(unused)]
                #[derive(Debug, Clone, Default, Deserialize)]
                struct Response {
                    total: usize,
                    skip: usize,
                    data: Vec<ClipboardMessage>,
                }
                let resp: Response = client.get(&url).query(&param).send().await?.json().await?;
                if cli.json {
                    println!("{}", serde_json::to_string(&resp.data)?);
                } else {
                    for record in resp.data {
                        match &record.entry.content {
                            client_interface::ServerClipboardContent::Text(text) => {
                                println!("{}", text);
                            }
                            client_interface::ServerClipboardContent::ImageUrl(image) => {
                                println!("{}", image);
                            }
                        }
                    }
                }
            }
        }
        Commands::SendText { text_or_file } => {
            let (client_id, sender, mut receiver, join_handler) = start_msg_client(&args).await?;
            if text_or_file.starts_with('@') {
                let path = text_or_file.trim_start_matches('@');
                let path = std::path::Path::new(if path.is_empty() { "/dev/stdin" } else { path });
                let text = std::fs::read_to_string(path)?;
                sender
                    .send(
                        client_interface::ClipboardRecord {
                            source: client_id,
                            content: client_interface::ClipboardContent::Text(text),
                        }
                        .into(),
                    )
                    .await?;
            } else {
                sender
                    .send(
                        client_interface::ClipboardRecord {
                            source: client_id,
                            content: client_interface::ClipboardContent::Text(text_or_file),
                        }
                        .into(),
                    )
                    .await?;
            }
            sender.send(None).await?;
            receiver.close();
            join_handler.await?;
        }
        Commands::SendImage { path } => {
            let (client_id, sender, mut receiver, join_handler) = start_msg_client(&args).await?;

            let path = path.unwrap_or_else(|| PathBuf::from("/dev/stdin"));
            let image = image::open(path)?.into_rgba8();
            sender
                .send(
                    client_interface::ClipboardRecord {
                        source: client_id,
                        content: client_interface::ClipboardContent::Image(ImageData {
                            width: image.width() as usize,
                            height: image.height() as usize,
                            data: image.into_raw(),
                        }),
                    }
                    .into(),
                )
                .await?;
            sender.send(None).await?;
            receiver.close();
            join_handler.await?;
        }
        Commands::Monitor {
            output,
            image_dir,
            timestamp,
            source,
            escape,
        } => {
            let (_, _, mut receiver, _) = start_msg_client(&args).await?;

            let output = output.unwrap_or_else(|| PathBuf::from("/dev/stdout"));
            let mut image_index = 0;
            let mut output = tokio::fs::File::create(output).await?;
            loop {
                let Some(record) = receiver.recv().await else {
                    break;
                };
                match record.content {
                    client_interface::ClipboardContent::Text(text) => {
                        let mut v = vec![];
                        if timestamp {
                            v.push(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                        }
                        if source {
                            v.push(record.source);
                        }
                        if escape {
                            v.push(serde_json::to_string(&text)?);
                        } else {
                            v.push(text);
                        }
                        let text = if escape { v.join("\t") } else { v.join("\n") };
                        output.write_all(text.as_bytes()).await?;
                        output.write_all(b"\n").await?;
                    }
                    client_interface::ClipboardContent::Image(image) => {
                        if let Some(image_dir) = &image_dir {
                            let path = image_dir.join(format!("{}.png", image_index));
                            image_index += 1;
                            let mut file = tokio::fs::File::create(path).await?;
                            file.write_all(&image.to_png()?).await?;
                        } else {
                            // Ignore image
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
