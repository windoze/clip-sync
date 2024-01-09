use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use client_interface::{ClipboardRecord, ClipboardSink, ClipboardSource};
use log::{debug, info, trace, warn};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::timeout,
};

/// Publish the received content to the sink.
pub(crate) async fn clipboard_publisher(
    mut sink: impl ClipboardSink,
    mut receiver: Receiver<Option<ClipboardRecord>>,
    shutdown: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), receiver.recv()).await {
            Ok(Some(data)) => {
                let Some(data) = data else {
                    info!("Received None from channel, closing");
                    shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
                    break;
                };
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
    shutdown: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    loop {
        match timeout(Duration::from_secs(1), source.poll()).await {
            Ok(Ok(clipboard_data)) => {
                debug!("Clipboard data = {:?}", clipboard_data);
                if clipboard_data.source == client_id {
                    debug!("Skipping clipboard update message sent by self");
                    continue;
                }
                sender.send(clipboard_data).await?;
            }
            Ok(Err(e)) => {
                warn!("Failed to receive clipboard data: {}", e);
                return Err(anyhow::anyhow!("Failed to receive clipboard data"));
            }
            Err(_) => {
                if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                    info!("Shutting down clipboard subscriber");
                    break;
                }
            }
        }
    }
    Ok(())
}
