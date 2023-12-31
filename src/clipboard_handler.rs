use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use arboard::Clipboard;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use futures::Future;
use log::{debug, warn};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{ClipboardContent, ClipboardData, ImageData};

pub trait ClipboardSource {
    fn poll(&mut self) -> impl Future<Output = anyhow::Result<String>>;
}

pub trait ClipboardSink {
    fn publish(&mut self, data: Option<String>) -> impl Future<Output = anyhow::Result<()>>;
}

pub struct Handler {
    pub sender: Sender<ClipboardData>,
    pub provider: Clipboard,
    pub sender_id: String,
    pub last_text: ClipboardContent,
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        debug!("Clipboard change happened!");
        if let Ok(Some(data)) = get_clipboard_content(&mut self.provider) {
            let data = ClipboardData {
                source: self.sender_id.clone(),
                data,
            };
            if data.data == self.last_text {
                debug!("Skipping clipboard update from self");
                return CallbackResult::Next;
            } else {
                self.last_text = data.data.clone();
            }
            self.sender.blocking_send(data).ok();
        }
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: std::io::Error) -> CallbackResult {
        warn!("Error: {}", error);
        CallbackResult::Next
    }
}

pub async fn start(
    sender_id: String,
    source: impl ClipboardSource,
    sink: impl ClipboardSink,
) -> anyhow::Result<()> {
    let mut provider = Clipboard::new().map_err(|e| {
        anyhow::anyhow!("Failed to initialize clipboard provider: {}", e.to_string())
    })?;
    let last_text = Arc::new(RwLock::new(
        get_clipboard_content(&mut provider)?.unwrap_or(ClipboardContent::Text("".to_string())),
    ));

    let (sender, receiver) = tokio::sync::mpsc::channel(10);

    let publisher_task = clipboard_publisher(sink, receiver, last_text.clone());
    let subscriber_task = clipboard_subscriber(source, sender_id.clone(), last_text.clone());

    let handler = Handler {
        sender,
        provider,
        sender_id,
        last_text: ClipboardContent::Text("".to_string()),
    };

    std::thread::spawn(move || {
        let _ = Master::new(handler).run();
    });
    let (r1, r2) = tokio::join!(publisher_task, subscriber_task);
    r1?;
    r2?;
    Ok(())
}

pub async fn clipboard_publisher(
    mut sink: impl ClipboardSink,
    mut receiver: Receiver<ClipboardData>,
    last_text: Arc<RwLock<ClipboardContent>>,
) -> anyhow::Result<()> {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await {
            Ok(Some(data)) => {
                if data.data == *last_text.read().unwrap() {
                    debug!("Skipping clipboard update from self");
                    continue;
                }
                let payload = serde_json::to_string(&data).unwrap();
                sink.publish(Some(payload)).await?;
            }
            Ok(None) => {
                debug!("Channel closed");
                break;
            }
            Err(_) => {
                debug!("Sending ping to server");
                sink.publish(None).await?;
            }
        }
    }
    Ok(())
}

pub async fn clipboard_subscriber(
    mut source: impl ClipboardSource,
    client_id: String,
    last_text: Arc<RwLock<ClipboardContent>>,
) -> anyhow::Result<()> {
    let mut provider = Clipboard::new().map_err(|e| {
        anyhow::anyhow!("Failed to initialize clipboard provider: {}", e.to_string())
    })?;

    loop {
        let payload = source.poll().await?;
        debug!("Received some payload");
        if let Ok(content) = serde_json::from_slice::<ClipboardData>(payload.as_bytes()) {
            debug!("Clipboard data = {:?}", content);
            if content.source == client_id {
                debug!("Skipping clipboard update from self");
                continue;
            }
            *last_text.write().unwrap() = content.data.clone();
            // HACK: Texts on Windows and macOS have different line endings, setting clipboard does auto-conversion and this caused the clipboard to be updated endlessly on both sides.
            // provider.set_text(content.data.replace("\r\n", "\n")).ok();
            set_clipboard_content(&mut provider, content.data).ok();
        } else {
            warn!("Failed to deserialize clipboard data");
        }
    }
}

fn get_clipboard_text(provider: &mut Clipboard) -> anyhow::Result<Option<String>> {
    match provider.get_text() {
        Ok(text) => {
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.replace("\r\n", "\n")))
            }
        }
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(e) => {
            debug!("Failed to get text from clipboard: {}", e);
            Err(anyhow::anyhow!("Failed to get text from clipboard"))
        }
    }
}

fn get_clipboard_image(provider: &mut Clipboard) -> anyhow::Result<Option<ImageData>> {
    match provider.get_image() {
        Ok(img) => {
            if img.bytes.len() == 0 {
                Ok(None)
            } else {
                Ok(Some(ImageData {
                    width: img.width,
                    height: img.height,
                    data: img.bytes.to_vec(),
                }))
            }
        }
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(e) => {
            debug!("Failed to get image from clipboard: {}", e);
            Err(anyhow::anyhow!("Failed to get image from clipboard"))
        }
    }
}

fn get_clipboard_content(provider: &mut Clipboard) -> anyhow::Result<Option<ClipboardContent>> {
    if let Some(text) = get_clipboard_text(provider)? {
        debug!("Got text from clipboard: {}", text);
        Ok(Some(ClipboardContent::Text(text)))
    } else if let Some(image) = get_clipboard_image(provider)? {
        debug!("Got image from clipboard.");
        Ok(Some(ClipboardContent::Image(image)))
    } else {
        debug!("Unsupported clipboard content");
        Ok(None)
    }
}

fn set_clipboard_content(
    provider: &mut Clipboard,
    content: ClipboardContent,
) -> anyhow::Result<()> {
    match content {
        ClipboardContent::Text(text) => {
            provider.set_text(text.replace("\r\n", "\n")).ok();
        }
        ClipboardContent::Image(image) => {
            provider
                .set_image(arboard::ImageData {
                    bytes: image.data.into(),
                    width: image.width,
                    height: image.height,
                })
                .ok();
        }
    }
    Ok(())
}
