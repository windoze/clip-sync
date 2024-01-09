#![windows_subsystem = "windows"]

use clap::Parser;
use client_interface::ClipSyncClient;
use log::info;
use platform_dirs::AppDirs;
use serde::Deserialize;

use std::path::PathBuf;

#[cfg(not(feature = "server-only"))]
mod clipboard_handler;

pub static APP_ICON: &[u8] = include_bytes!("../icons/app-icon.png");

#[cfg(not(feature = "server-only"))]
pub use client_interface::{ClipboardSink, ClipboardSource};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Args {
    pub roles: Vec<String>,
    #[cfg(feature = "server")]
    #[serde(default)]
    pub server: websocket_server::ServerConfig,
    #[cfg(feature = "mqtt")]
    #[serde(default)]
    pub mqtt_client: mqtt_client::MqttClientConfig,
    #[cfg(feature = "websocket")]
    #[serde(default)]
    pub websocket_client: websocket_client::ClientConfig,
}

impl Args {
    #[cfg(all(feature = "tray", feature = "websocket"))]
    pub fn get_server_url(&self) -> Option<String> {
        if self.roles.contains(&"websocket-client".to_string()) {
            if let Ok(mut url) = url::Url::parse(&self.websocket_client.server_url) {
                let scheme = if url.scheme() == "wss" {
                    "https"
                } else if url.scheme() == "ws" {
                    "http"
                } else {
                    return None;
                };
                url.set_scheme(scheme).unwrap();
                url.set_path("");
                Some(url.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn get_config_file() -> PathBuf {
    let app_dirs = AppDirs::new(Some(env!("CARGO_PKG_NAME")), false).unwrap();
    app_dirs.config_dir.join("config.toml")
}

async fn svc_main(args: Args) -> anyhow::Result<()> {
    let mut tasks: Vec<tokio::task::JoinHandle<anyhow::Result<()>>> = vec![];
    #[cfg(feature = "server")]
    if args.roles.contains(&"server".to_string()) {
        tasks.push(tokio::spawn(async {
            info!("Starting websocket server");
            websocket_server::server_main(args.server)
                .await
                .map_err(|e| anyhow::anyhow!("Server error: {}", e))
        }));
    }
    #[cfg(feature = "mqtt")]
    if args.roles.contains(&"mqtt-client".to_string()) {
        tasks.push(tokio::spawn(async {
            info!("Starting MQTT client");
            let (sender_id, source, sink) =
                mqtt_client::MqttClipSyncClient::connect(args.mqtt_client).await?;
            clipboard_handler::start(sender_id, source, sink).await
        }));
    }
    #[cfg(feature = "websocket")]
    if args.roles.contains(&"websocket-client".to_string()) {
        tasks.push(tokio::spawn(async {
            info!("Starting websocket client");
            let (sender_id, source, sink) =
                websocket_client::WebsocketClipSyncClient::connect(args.websocket_client).await?;
            clipboard_handler::start(sender_id, source, sink).await
        }));
    }
    if args.roles.is_empty() {
        anyhow::bail!("No role specified");
    }
    for r in futures::future::join_all(tasks.into_iter())
        .await
        .into_iter()
    {
        r??;
    }
    Ok(())
}

#[cfg(feature = "tray")]
mod tray {
    use tray_item::{IconSource, TrayItem};

    #[cfg(target_os = "macos")]
    fn get_app_icon() -> IconSource {
        IconSource::Data {
            width: 0,
            height: 0,
            data: crate::APP_ICON.to_vec(),
        }
    }

    #[cfg(target_os = "windows")]
    fn get_app_icon() -> IconSource {
        IconSource::Resource("default")
    }

    #[cfg(target_os = "linux")]
    fn get_app_icon() -> IconSource {
        let decoder = png::Decoder::new(std::io::Cursor::new(crate::APP_ICON));
        let mut reader = decoder.read_info().expect("Failed to decode icon");
        let info = reader.info();
        let mut buf = vec![0; info.raw_bytes()];
        let output_info = reader
            .next_frame(buf.as_mut_slice())
            .expect("Failed to decode icon");
        output_info.buffer_size();

        IconSource::Data {
            width: output_info.width as i32,
            height: output_info.height as i32,
            data: buf[0..output_info.buffer_size()].to_vec(),
        }
    }

    pub fn run_tray(
        #[cfg(feature = "websocket")] server_url: Option<String>,
    ) -> anyhow::Result<()> {
        let mut tray = TrayItem::new("ClipSync", get_app_icon())?;

        #[cfg(target_os = "macos")]
        {
            #[cfg(feature = "websocket")]
            tray.inner_mut().add_menu_item("Open Portal", move || {
                if let Some(url) = &server_url {
                    webbrowser::open(url).ok();
                }
            })?;
            tray.inner_mut().add_quit_item("Quit");
            tray.inner_mut().display();
        }

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            enum Message {
                Portal,
                Quit,
            }
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            let tx_clone = tx.clone();
            #[cfg(feature = "websocket")]
            tray.add_menu_item("Open Portal", move || {
                tx_clone.send(Message::Portal).unwrap();
            })?;
            tray.add_menu_item("Quit", move || {
                tx.send(Message::Quit).unwrap();
            })?;
            #[allow(clippy::while_let_loop)] // In case we want to add more menu items
            loop {
                match rx.recv()? {
                    Message::Portal =>
                    {
                        #[cfg(feature = "websocket")]
                        if let Some(url) = &server_url {
                            webbrowser::open(url).ok();
                        }
                    }
                    Message::Quit => {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    #[derive(Debug, Clone, Parser)]
    struct Config {
        #[arg(long = "config")]
        config_path: Option<std::path::PathBuf>,
        #[cfg(not(feature = "server-only"))]
        #[arg(long, default_value = "false")]
        no_tray: bool,
        #[command(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    }

    let cli = Config::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .filter_module("tantivy", log::LevelFilter::Warn) // Tantivy is too talky at the INFO level
        .init();
    info!("Starting");
    let config_path = cli.config_path.unwrap_or(get_config_file());
    let config = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Failed to read config at '{:?}'", config_path));
    let args = toml::from_str::<Args>(&config)?;

    #[cfg(all(feature = "tray", feature = "websocket"))]
    let server_url = args.get_server_url();

    let join_handler = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        runtime
            .block_on(svc_main(args))
            .expect("Failed to run service");
    });

    #[cfg(feature = "tray")]
    {
        if cli.no_tray {
            join_handler.join().unwrap();
        } else {
            tray::run_tray(
                #[cfg(feature = "websocket")]
                server_url,
            )?;
        }
    }
    #[cfg(not(feature = "tray"))]
    {
        join_handler.join().unwrap();
    }

    Ok(())
}
