use std::time::Duration;

use client_interface::{ClipboardRecord, ClipboardSink, ClipboardSource};
use log::{debug, trace, warn};
use tokio::sync::mpsc::{Receiver, Sender};

/// Publish the received content to the sink.
pub(crate) async fn clipboard_publisher(
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

/// Poll the clipboard content from the source and send it to sender.
pub(crate) async fn clipboard_subscriber(
    mut source: impl ClipboardSource,
    sender: Sender<ClipboardRecord>,
    client_id: String,
) -> anyhow::Result<()> {
    loop {
        if let Ok(clipboard_data) = source.poll().await {
            debug!("Clipboard data = {:?}", clipboard_data);
            if clipboard_data.source == client_id {
                debug!("Skipping clipboard update message sent by self");
                continue;
            }
            sender.send(clipboard_data).await?;
        } else {
            warn!("Failed to receive clipboard data");
            return Err(anyhow::anyhow!("Failed to receive clipboard data"));
        }
    }
}
