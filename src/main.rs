#![windows_subsystem = "windows"]

use clap::Parser;
use futures::{future::join_all, TryFutureExt};
use platform_dirs::AppDirs;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tray_item::{IconSource, TrayItem};

use std::path::PathBuf;

mod client;
mod clipboard_handler;
mod mqtt_client;

pub use clipboard_handler::{ClipboardSink, ClipboardSource};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Args {
    pub roles: Vec<String>,
    pub server: server::ServerConfig,
    pub mqtt_client: mqtt_client::MqttClientConfig,
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

    let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = vec![];
    if args.roles.contains(&"server".to_string()) {
        tasks.push(tokio::spawn(async {
            server::server_main(args.server)
                .map_err(|e| anyhow::anyhow!("Server error: {}", e))
                .await
        }));
    }
    if args.roles.contains(&"mqtt-client".to_string()) {
        tasks.push(tokio::spawn(async {
            mqtt_client::clip_sync_svc(args.mqtt_client)
                .map_err(|e| anyhow::anyhow!("Mqtt client error: {}", e))
                .await
        }));
    }
    if args.roles.contains(&"websocket-client".to_string()) {
        tasks.push(tokio::spawn(async {
            client::clip_sync_svc(args.websocket_client)
                .map_err(|e| anyhow::anyhow!("Mqtt client error: {}", e))
                .await
        }));
    }
    if args.roles.is_empty() {
        anyhow::bail!("No role specified");
    }
    for r in join_all(tasks.into_iter()).await.into_iter() {
        r??;
    }

    Ok(())
}

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
    let decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!(
        "../icons/app-icon.png"
    )));
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

fn main() -> anyhow::Result<()> {
    #[derive(Debug, Clone, Parser)]
    struct Config {
        #[arg(long = "config")]
        config_path: Option<std::path::PathBuf>,
        #[arg(long, default_value = "false")]
        no_tray: bool,
        #[command(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    }

    let cli = Config::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();
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

    if cli.no_tray {
        join_handler.join().unwrap();
        return Ok(());
    }

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
                warn!("Quit");
                break;
            }
        }
    }
    Ok(())
}
