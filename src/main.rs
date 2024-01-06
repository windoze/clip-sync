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

#[cfg(not(feature = "server-only"))]
impl Args {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardRecord {
    pub source: String,
    pub content: ClipboardContent,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageData {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl ImageData {
    pub fn from_png(bytes: &[u8]) -> anyhow::Result<Self> {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("Failed to decode icon");
        let info = reader.info();
        let mut buf = vec![0; info.raw_bytes()];
        let output_info = reader
            .next_frame(buf.as_mut_slice())
            .expect("Failed to decode icon");
        output_info.buffer_size();

        Ok(Self {
            width: output_info.width as usize,
            height: output_info.height as usize,
            data: buf[0..output_info.buffer_size()].to_vec(),
        })
    }

    pub fn to_png(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        let mut encoder = png::Encoder::new(&mut buf, self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.data)?;
        writer.finish()?;
        Ok(buf)
    }
}

impl std::fmt::Debug for ImageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardContent {
    Text(String),
    Image(ImageData),
}

impl ClipboardContent {
    pub fn clear(&mut self) {
        *self = ClipboardContent::Text("".to_string());
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ClipboardContent::Text(text) => text.is_empty(),
            ClipboardContent::Image(img) => img.data.is_empty(),
        }
    }
}

impl std::fmt::Debug for ClipboardContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardContent::Text(text) => write!(f, "Text({})", text),
            ClipboardContent::Image(img) => write!(f, "Image({}x{})", img.width, img.height),
        }
    }
}

fn get_config_file() -> PathBuf {
    let app_dirs = AppDirs::new(Some(env!("CARGO_PKG_NAME")), false).unwrap();
    app_dirs.config_dir.join("config.toml")
}

async fn svc_main(args: Args) -> anyhow::Result<()> {
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

    pub fn run_tray(server_url: Option<String>) -> anyhow::Result<()> {
        let mut tray = TrayItem::new("ClipSync", get_app_icon())?;

        #[cfg(target_os = "macos")]
        {
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
            tray.add_menu_item("Open Portal", move || {
                tx_clone.send(Message::Portal).unwrap();
            })?;
            tray.add_menu_item("Quit", move || {
                tx.send(Message::Quit).unwrap();
            })?;
            #[allow(clippy::while_let_loop)] // In case we want to add more menu items
            loop {
                match rx.recv()? {
                    Message::Portal => {
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
    #[cfg(not(feature = "server-only"))]
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

        tray::run_tray(server_url)?;
    }

    Ok(())
}
