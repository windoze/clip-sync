#![windows_subsystem = "windows"]

use clap::Parser;
use futures::TryFutureExt;
use log::info;
use platform_dirs::AppDirs;
use serde::{Deserialize, Serialize};

use std::path::PathBuf;

#[cfg(not(feature = "server-only"))]
mod client;
#[cfg(not(feature = "server-only"))]
mod clipboard_handler;
#[cfg(not(feature = "server-only"))]
mod mqtt_client;
mod server;

pub static APP_ICON: &[u8] = include_bytes!("../icons/app-icon.png");

#[cfg(not(feature = "server-only"))]
pub use clipboard_handler::{ClipboardSink, ClipboardSource};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Args {
    #[cfg(not(feature = "server-only"))]
    pub roles: Vec<String>,
    #[serde(default)]
    pub server: server::ServerConfig,
    #[cfg(not(feature = "server-only"))]
    #[serde(default)]
    pub mqtt_client: mqtt_client::MqttClientConfig,
    #[cfg(not(feature = "server-only"))]
    #[serde(default)]
    pub websocket_client: client::ClientConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipboardData {
    pub source: String,
    pub data: String,
}

fn get_config_file() -> PathBuf {
    let app_dirs = AppDirs::new(Some(env!("CARGO_PKG_NAME")), false).unwrap();
    app_dirs.config_dir.join("config.toml")
}

async fn svc_main(config_path: PathBuf) -> anyhow::Result<()> {
    let config = std::fs::read_to_string(config_path)?;
    let args = toml::from_str::<Args>(&config)?;

    #[cfg(not(feature = "server-only"))]
    {
        let mut tasks: Vec<tokio::task::JoinHandle<anyhow::Result<()>>> = vec![];
        if args.roles.contains(&"server".to_string()) {
            tasks.push(tokio::spawn(async {
                info!("Starting websocket server");
                server::server_main(args.server)
                    .map_err(|e| anyhow::anyhow!("Server error: {}", e))
                    .await
            }));
        }
        if args.roles.contains(&"mqtt-client".to_string()) {
            tasks.push(tokio::spawn(async {
                info!("Starting MQTT client");
                mqtt_client::clip_sync_svc(args.mqtt_client)
                    .map_err(|e| anyhow::anyhow!("Mqtt client error: {}", e))
                    .await
            }));
        }
        if args.roles.contains(&"websocket-client".to_string()) {
            tasks.push(tokio::spawn(async {
                info!("Starting websocket client");
                client::clip_sync_svc(args.websocket_client)
                    .map_err(|e| anyhow::anyhow!("Mqtt client error: {}", e))
                    .await
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
    }
    #[cfg(feature = "server-only")]
    {
        info!("Starting websocket server");
        server::server_main(args.server)
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
            .await?;
    }
    Ok(())
}

#[cfg(not(feature = "server-only"))]
mod tray {
    use tray_item::{IconSource, TrayItem};

    #[cfg(target_os = "macos")]
    fn get_app_icon() -> IconSource {
        IconSource::Data {
            width: 0,
            height: 0,
            data: include_bytes!("../icons/app-icon.png").to_vec(),
        }
    }

    #[cfg(target_os = "windows")]
    fn get_app_icon() -> IconSource {
        IconSource::Resource("default")
    }

    #[cfg(target_os = "linux")]
    fn get_app_icon() -> IconSource {
        let decoder = png::Decoder::new(std::io::Cursor::new(APP_ICON));
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

    pub fn run_tray() -> anyhow::Result<()> {
        let mut tray = TrayItem::new("ClipSync", get_app_icon())?;

        #[cfg(target_os = "macos")]
        {
            tray.inner_mut().add_quit_item("Quit");
            tray.inner_mut().display();
        }

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            enum Message {
                Quit,
            }
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            tray.add_menu_item("Quit", move || {
                tx.send(Message::Quit).unwrap();
            })?;
            loop {
                if matches!(rx.recv()?, Message::Quit) {
                    log::warn!("Quit");
                    break;
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
        .init();
    info!("Starting");
    let config_path = cli.config_path.unwrap_or(get_config_file());
    let join_handler = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        runtime
            .block_on(svc_main(config_path))
            .expect("Failed to run service");
    });

    #[cfg(feature = "server-only")]
    {
        join_handler.join().unwrap();
    }

    #[cfg(not(feature = "server-only"))]
    {
        if cli.no_tray {
            join_handler.join().unwrap();
            return Ok(());
        }

        tray::run_tray()?;
    }

    Ok(())
}
