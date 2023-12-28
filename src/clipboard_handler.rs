use std::sync::{Arc, RwLock};

use clipboard::{ClipboardContext, ClipboardProvider};
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use futures::Future;
use log::{debug, warn};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::ClipboardData;

pub trait ClipboardSource {
    fn poll(&mut self) -> impl Future<Output = anyhow::Result<String>>;
}

pub trait ClipboardSink {
    fn publish(&mut self, data: String) -> impl Future<Output = anyhow::Result<()>>;
}

pub struct Handler<T: ClipboardProvider> {
    pub sender: Sender<ClipboardData>,
    pub provider: T,
    pub sender_id: String,
    pub last_text: String,
}

impl<T: ClipboardProvider> ClipboardHandler for Handler<T> {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        debug!("Clipboard change happened!");
        let data = self
            .provider
            .get_contents()
            .unwrap_or_default()
            .replace("\r\n", "\n");
        let data = ClipboardData {
            source: self.sender_id.clone(),
            data,
        };
        if data.data.is_empty() || data.data == self.last_text {
            debug!("Skipping clipboard update from self");
            return CallbackResult::Next;
        } else {
            self.last_text = data.data.clone();
        }
        self.sender.blocking_send(data).ok();
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
    let mut provider: ClipboardContext = ClipboardProvider::new().map_err(|e| {
        anyhow::anyhow!("Failed to initialize clipboard provider: {}", e.to_string())
    })?;
    let last_text = Arc::new(RwLock::new(
        provider.get_contents().unwrap_or("".to_string()),
    ));

    let (sender, receiver) = tokio::sync::mpsc::channel(10);

    let publisher_task = clipboard_publisher(sink, receiver, last_text.clone());
    let subscriber_task = clipboard_subscriber(source, sender_id.clone(), last_text.clone());

    let handler = Handler {
        sender,
        provider,
        sender_id,
        last_text: "".to_string(),
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
    last_text: Arc<RwLock<String>>,
) -> anyhow::Result<()> {
    while let Some(data) = receiver.recv().await {
        if data.data.is_empty() || data.data == *last_text.read().unwrap() {
            debug!("Skipping clipboard update from self");
            continue;
        }
        let payload = serde_json::to_string(&data).unwrap();
        sink.publish(payload).await.ok();
    }
    Ok(())
}

pub async fn clipboard_subscriber(
    mut source: impl ClipboardSource,
    client_id: String,
    last_text: Arc<RwLock<String>>,
) -> anyhow::Result<()> {
    let mut provider: ClipboardContext = ClipboardProvider::new().map_err(|e| {
        anyhow::anyhow!("Failed to initialize clipboard provider: {}", e.to_string())
    })?;

    while let Ok(payload) = source.poll().await {
        debug!("Received = {:?}", payload);
        if let Ok(content) = serde_json::from_slice::<ClipboardData>(payload.as_bytes()) {
            debug!("Clipboard data = {:?}", content);
            if content.source == client_id {
                debug!("Skipping clipboard update from self");
                continue;
            }
            *last_text.write().unwrap() = content.data.clone();
            // HACK: Texts on Windows and macOS have different line endings, setting clipboard does auto-conversion and this caused the clipboard to be updated endlessly on both sides.
            provider
                .set_contents(content.data.replace("\r\n", "\n"))
                .ok();
        } else {
            warn!("Failed to deserialize clipboard data");
        }
    }
    Ok(())
}
