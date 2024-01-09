use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use arboard::Clipboard;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use log::{debug, info, trace, warn};
use tokio::sync::mpsc::{Receiver, Sender};

use client_interface::{
    ClipboardContent, ClipboardRecord, ClipboardSink, ClipboardSource, ImageData,
};

pub struct Handler {
    pub sender: Sender<ClipboardRecord>,
    pub provider: Clipboard,
    pub sender_id: String,
    pub last_set_content: Arc<Mutex<ClipboardContent>>,
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        debug!("Clipboard change happened!");
        if let Ok(Some(content)) = get_clipboard_content(&mut self.provider) {
            {
                let mut guard = self.last_set_content.lock().unwrap();
                if *guard == content {
                    debug!("Skipping clipboard update from self");
                    return CallbackResult::Next;
                }
                *guard = content.clone();
            }
            let data = ClipboardRecord {
                source: self.sender_id.clone(),
                content,
            };
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
    let last_set_content: Arc<Mutex<ClipboardContent>> =
        Arc::new(Mutex::new(ClipboardContent::Text("".to_string())));

    let provider = Clipboard::new().map_err(|e| {
        anyhow::anyhow!("Failed to initialize clipboard provider: {}", e.to_string())
    })?;
    let (sender, receiver) = tokio::sync::mpsc::channel(10);

    let publisher_task = clipboard_publisher(sink, receiver);
    let subscriber_task = clipboard_subscriber(source, sender_id.clone(), last_set_content.clone());

    let handler = Handler {
        sender,
        provider,
        sender_id: sender_id.clone(),
        last_set_content,
    };

    std::thread::spawn(move || {
        let _ = Master::new(handler).run();
    });
    let (r1, r2) = tokio::join!(publisher_task, subscriber_task);
    r1?;
    r2?;
    Ok(())
}

/// Publish the clipboard content to the sink if it's changed.
async fn clipboard_publisher(
    mut sink: impl ClipboardSink,
    mut receiver: Receiver<ClipboardRecord>,
) -> anyhow::Result<()> {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await {
            Ok(Some(data)) => {
                sink.publish(Some(data)).await?;
            }
            Ok(None) => {
                warn!("Channel closed");
                break;
            }
            Err(_) => {
                trace!("Sending ping to server");
                sink.publish(None).await?;
            }
        }
    }
    Err(anyhow::anyhow!("Publisher source channel closed"))
}

/// Poll the clipboard content from the source and set it to the system clipboard.
async fn clipboard_subscriber(
    mut source: impl ClipboardSource,
    client_id: String,
    last_set_content: Arc<Mutex<ClipboardContent>>,
) -> anyhow::Result<()> {
    loop {
        if let Ok(clipboard_data) = source.poll().await {
            debug!("Clipboard data = {:?}", clipboard_data);
            if clipboard_data.source == client_id {
                debug!("Skipping clipboard update message sent by self");
                continue;
            }
            Clipboard::new()
                .map_err(|e| {
                    anyhow::anyhow!("Failed to initialize clipboard provider: {}", e.to_string())
                })
                .and_then(|mut provider| {
                    set_clipboard_content(&mut provider, clipboard_data.content.clone()).map(
                        |changed| {
                            if changed {
                                info!("Clipboard updated");
                                *last_set_content.lock().unwrap() = clipboard_data.content;
                            }
                        },
                    )
                })
                .ok();
        } else {
            warn!("Failed to receive clipboard data");
            return Err(anyhow::anyhow!("Failed to receive clipboard data"));
        }
    }
}

fn get_clipboard_text(provider: &mut Clipboard) -> anyhow::Result<Option<String>> {
    match provider.get_text() {
        Ok(text) => {
            if text.is_empty() {
                Ok(None)
            } else {
                // HACK: Windows and macOS/Linux have different line endings.
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
        debug!("Got image from clipboard {:?}.", image);
        Ok(Some(ClipboardContent::Image(image)))
    } else {
        debug!("Unsupported clipboard content");
        Ok(None)
    }
}

fn set_clipboard_content(
    provider: &mut Clipboard,
    content: ClipboardContent,
) -> anyhow::Result<bool> {
    let existing = match &content {
        ClipboardContent::Text(_) => get_clipboard_text(provider)?.map(ClipboardContent::Text),
        ClipboardContent::Image(_) => get_clipboard_image(provider)?.map(ClipboardContent::Image),
    };
    if let Some(existing) = existing {
        if existing == content {
            debug!("Clipboard content unchanged, skipping");
            return Ok(false);
        }
    }
    provider.clear()?;
    match content {
        // HACK: Windows and macOS/Linux have different line endings.
        ClipboardContent::Text(text) => provider.set_text(text.replace("\r\n", "\n")),
        ClipboardContent::Image(image) => provider.set_image(arboard::ImageData {
            bytes: image.data.into(),
            width: image.width,
            height: image.height,
        }),
    }?;
    Ok(true)
}
