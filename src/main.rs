#![windows_subsystem = "windows"]

use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use env_logger::Env;
use gethostname::gethostname;
use log::{debug, warn};
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use tray_item::{IconSource, TrayItem};

use std::{
    io,
    sync::{atomic::AtomicBool, Arc},
};

#[derive(Debug, Clone, Parser)]
struct Args {
    #[clap(short = 'a', long)]
    pub mqtt_server_addr: String,
    #[clap(short = 'p', long)]
    pub mqtt_server_port: u16,
    #[clap(short = 't', long, default_value = "clipboard")]
    pub mqtt_topic: String,
    #[clap(short = 'u', long)]
    pub mqtt_username: Option<String>,
    #[clap(short = 'w', long)]
    pub mqtt_password: Option<String>,
    #[clap(short = 'c', long)]
    pub mqtt_client_id: Option<String>,
}

struct Handler<T: ClipboardProvider> {
    sender: Sender<ClipboardData>,
    provider: T,
    sender_id: String,
    last_text: String,
    clip_monitor_flag: Arc<AtomicBool>,
}

impl<T: ClipboardProvider> ClipboardHandler for Handler<T> {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        debug!("Clipboard change happened!");
        if !self
            .clip_monitor_flag
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            debug!("Skipping clipboard change event");
            return CallbackResult::Next;
        }
        let data = self.provider.get_contents().unwrap_or_default();
        if data.is_empty() || data.replace("\r\n", "\n") == self.last_text.replace("\r\n", "\n") {
            return CallbackResult::Next;
        }
        self.last_text = data.clone();
        let data = ClipboardData {
            source: self.sender_id.clone(),
            data,
        };
        self.sender.blocking_send(data).ok();
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("Error: {}", error);
        CallbackResult::Next
    }
}

async fn clipboard_publisher(
    client: AsyncClient,
    mut receiver: Receiver<ClipboardData>,
    topic: String,
) -> anyhow::Result<()> {
    while let Some(data) = receiver.recv().await {
        let payload = serde_json::to_string(&data).unwrap();
        client
            .publish(topic.clone(), QoS::AtLeastOnce, false, payload)
            .await
            .ok();
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct ClipboardData {
    source: String,
    data: String,
}

async fn clipboard_subscriber(
    mut eventloop: EventLoop,
    clip_monitor_flag: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut provider: ClipboardContext = ClipboardProvider::new().map_err(|e| {
        anyhow::anyhow!("Failed to initialize clipboard provider: {}", e.to_string())
    })?;

    while let Ok(notification) = eventloop.poll().await {
        debug!("Received = {:?}", notification);
        if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(p)) = notification {
            if let Ok(content) = serde_json::from_slice::<ClipboardData>(&p.payload) {
                debug!("Clipboard data = {:?}", content);
                clip_monitor_flag.store(false, std::sync::atomic::Ordering::Relaxed);
                // HACK: Texts on Windows and macOS have different line endings, setting clipboard does auto-conversion and this caused the clipboard to be updated endlessly on both sides.
                provider
                    .set_contents(content.data.replace("\r\n", "\n"))
                    .ok();
                clip_monitor_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            } else {
                warn!("Failed to deserialize clipboard data");
            }
        }
    }
    Ok(())
}

async fn svc_main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    let args = Args::parse();

    let clip_monitor_flag = Arc::new(AtomicBool::new(true));

    let sender_id = args.mqtt_client_id.unwrap_or(
        gethostname()
            .into_string()
            .unwrap_or("<unknown>".to_string()),
    );
    let mut options = MqttOptions::new(
        sender_id.clone(),
        args.mqtt_server_addr,
        args.mqtt_server_port,
    );

    if args.mqtt_username.is_some() {
        options.set_credentials(
            args.mqtt_username.unwrap(),
            args.mqtt_password.unwrap_or("".to_string()),
        );
    }

    let (sender, receiver) = tokio::sync::mpsc::channel(10);
    let (client, eventloop) = AsyncClient::new(options, 10);
    client
        .subscribe(args.mqtt_topic.clone(), QoS::AtLeastOnce)
        .await?;

    let publisher_task = clipboard_publisher(client, receiver, args.mqtt_topic);
    let subscriber_task = clipboard_subscriber(eventloop, clip_monitor_flag.clone());

    let mut provider: ClipboardContext = ClipboardProvider::new().map_err(|e| {
        anyhow::anyhow!("Failed to initialize clipboard provider: {}", e.to_string())
    })?;
    let last_text = provider.get_contents().unwrap_or("".to_string());
    let handler = Handler {
        sender,
        provider,
        sender_id,
        last_text,
        clip_monitor_flag,
    };

    std::thread::spawn(move || {
        let _ = Master::new(handler).run();
    });
    let (r1, r2) = tokio::join!(publisher_task, subscriber_task);
    r1?;
    r2?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn main() {
    let icon = IconSource::Data {
        width: 0,
        height: 0,
        data: include_bytes!("../icons/app-icon.png").to_vec(),
    };
    let mut tray = TrayItem::new("ClipSync", icon).unwrap();

    let inner = tray.inner_mut();
    inner.add_quit_item("Quit");

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(svc_main()).expect("Failed to run service");
    });

    inner.display();
}

#[cfg(target_os = "windows")]
fn main() {
    enum Message {
        Quit,
    }
    let icon = IconSource::Resource("default");
    let mut tray = TrayItem::new("ClipSync", icon).unwrap();

    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    tray.add_menu_item("Quit", move || {
        tx.send(Message::Quit).unwrap();
    })
    .unwrap();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(svc_main()).expect("Failed to run service");
    });

    loop {
        match rx.recv() {
            Ok(Message::Quit) => {
                warn!("Quit");
                break;
            }
            _ => {}
        }
    }
}

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use std::io::Cursor;

    enum Message {
        Quit,
    }

    let decoder = png::Decoder::new(Cursor::new(include_bytes!("../icons/app-icon.png")));
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    let mut buf = vec![0; info.raw_bytes()];
    let output_info = reader.next_frame(buf.as_mut_slice())?;
    output_info.buffer_size();

    let icon = IconSource::Data {
        width: output_info.width as i32,
        height: output_info.height as i32,
        data: buf[0..output_info.buffer_size()].to_vec(),
    };
    let mut tray = TrayItem::new("ClipSync", icon).unwrap();

    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    tray.add_menu_item("Quit", move || {
        tx.send(Message::Quit).unwrap();
    })
    .unwrap();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(svc_main()).expect("Failed to run service");
    });

    loop {
        if matches!(rx.recv()?, Message::Quit) {
            warn!("Quit");
            break;
        }
    }
    Ok(())
}
