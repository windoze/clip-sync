use std::path::PathBuf;

use clap::{Parser, Subcommand};

use client_interface::{ClipSyncClient, ImageData};
use tokio::io::AsyncWriteExt;

mod client;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Send text to the server
    #[command(arg_required_else_help = true)]
    Send {
        /// Text to send, or path to file to send if prefixed with '@'
        #[clap(index = 1)]
        text_or_file: String,
    },
    /// Send image to the server
    #[command(arg_required_else_help = true)]
    SendImage {
        /// Path to image file to send, omit to read from stdin
        #[clap(index = 1)]
        path: Option<PathBuf>,
    },
    /// Monitor clipboard content
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
    /// Verbosity level
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    /// Subcommand
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .filter_module("tantivy", log::LevelFilter::Warn) // Tantivy is too talky at the INFO level
        .init();
    let mut args = clip_sync_config::parse_config(cli.config_path)?;
    let (sender, clipboard_publisher_receiver) = tokio::sync::mpsc::channel(10);
    let (clipboard_subscriber_sender, mut receiver) = tokio::sync::mpsc::channel(10);

    let client_id = if args.roles.contains(&("websocket-client").to_string()) {
        args.websocket_client.client_id = Some("$cli".to_string());
        let (client_id, source, sink) =
            websocket_client::WebsocketClipSyncClient::connect(args.websocket_client).await?;
        let client_id_clone = client_id.clone();
        tokio::spawn(async move {
            let publisher_task = client::clipboard_publisher(sink, clipboard_publisher_receiver);
            let subscriber_task =
                client::clipboard_subscriber(source, clipboard_subscriber_sender, client_id_clone);

            let (r1, r2) = tokio::join!(publisher_task, subscriber_task);
            r1.unwrap();
            r2.unwrap();
        });
        client_id
    } else {
        #[cfg(feature = "mqtt")]
        if args.roles.contains(&("mqtt-client").to_string()) {
            args.websocket_client.client_id = Some("$cli".to_string());
            let (client_id, source, sink) =
                mqtt_client::MqttClipSyncClient::connect(args.mqtt_client).await?;
            let client_id_clone = client_id.clone();
            tokio::spawn(async move {
                let publisher_task =
                    client::clipboard_publisher(sink, clipboard_publisher_receiver);
                let subscriber_task = client::clipboard_subscriber(
                    source,
                    clipboard_subscriber_sender,
                    client_id_clone,
                );

                let (r1, r2) = tokio::join!(publisher_task, subscriber_task);
                r1.unwrap();
                r2.unwrap();
            });
            client_id
        } else {
            panic!("No client role specified");
        }
        #[cfg(not(feature = "mqtt"))]
        panic!("No client role specified");
    };
    match cli.command {
        Commands::Send { text_or_file } => {
            if text_or_file.starts_with('@') {
                let path = text_or_file.trim_start_matches('@');
                let path = std::path::Path::new(if path.is_empty() { "/dev/stdin" } else { path });
                let text = std::fs::read_to_string(path)?;
                sender
                    .send(client_interface::ClipboardRecord {
                        source: client_id,
                        content: client_interface::ClipboardContent::Text(text),
                    })
                    .await?;
            } else {
                sender
                    .send(client_interface::ClipboardRecord {
                        source: client_id,
                        content: client_interface::ClipboardContent::Text(text_or_file),
                    })
                    .await?;
            }
        }
        Commands::SendImage { path } => {
            let path = path.unwrap_or_else(|| PathBuf::from("/dev/stdin"));
            let image = image::open(path)?.into_rgba8();
            sender
                .send(client_interface::ClipboardRecord {
                    source: client_id,
                    content: client_interface::ClipboardContent::Image(ImageData {
                        width: image.width() as usize,
                        height: image.height() as usize,
                        data: image.into_raw(),
                    }),
                })
                .await?;
        }
        Commands::Monitor {
            output,
            image_dir,
            timestamp,
            source,
            escape,
        } => {
            let output = output.unwrap_or_else(|| PathBuf::from("/dev/stdout"));
            let mut image_index = 0;
            let mut output = tokio::fs::File::create(output).await?;
            loop {
                let record = receiver.recv().await.unwrap();
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
