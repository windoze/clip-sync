#![windows_subsystem = "windows"]

use client_interface::ClipSyncClient;
use clip_sync_config::Args;
use log::info;

pub use client_interface::{ClipboardSink, ClipboardSource};

mod clipboard_handler;

pub static APP_ICON: &[u8] = include_bytes!("../icons/app-icon.png");

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
    let args = clip_sync_config::parse()?;

    #[cfg(all(feature = "tray", feature = "websocket"))]
    let server_url = args.get_server_url();

    #[allow(unused_variables)]
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
        tray::run_tray(
            #[cfg(feature = "websocket")]
            server_url,
        )?;
    }

    #[cfg(not(feature = "tray"))]
    {
        join_handler.join().unwrap();
    }

    Ok(())
}
