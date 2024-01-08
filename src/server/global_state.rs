use std::{collections::HashSet, path::PathBuf};

use log::{debug, info, warn};
use tokio::{
    runtime::{Builder, Handle},
    sync::broadcast::Sender,
};

use super::{
    search::Search, ClipboardMessage, QueryParam, QueryResult, ServerClipboardContent, ServerConfig,
};

pub struct GlobalState {
    sender: Sender<ClipboardMessage>,
    device_list: HashSet<String>,
    online_device_list: HashSet<String>,
    search: Search,
    _rt: tokio::runtime::Runtime,
    thread_pool: Handle,
    image_path: PathBuf,
}

impl GlobalState {
    pub fn new(args: &ServerConfig, sender: Sender<ClipboardMessage>) -> Self {
        let rt = Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("search-pool")
            .build()
            .unwrap();

        let handle = rt.handle().clone();

        let search = Search::new(args.index_path.clone());
        let device_list = search.get_device_list().unwrap();

        Self {
            sender,
            device_list,
            online_device_list: HashSet::new(),
            search,
            _rt: rt,
            thread_pool: handle,
            image_path: args.image_path.clone().unwrap(),
        }
    }

    pub fn get_image_path(&self) -> &PathBuf {
        &self.image_path
    }

    pub fn get_receiver(&self) -> tokio::sync::broadcast::Receiver<ClipboardMessage> {
        self.sender.subscribe()
    }

    pub fn add_device(&mut self, name: impl ToString) {
        self.device_list.insert(name.to_string());
        if self.online_device_list.insert(name.to_string()) {
            info!("Device '{}' added.", name.to_string());
        }
    }

    pub fn remove_device(&mut self, name: impl AsRef<str>) {
        if self.online_device_list.remove(name.as_ref()) {
            info!("Device '{}' removed.", name.as_ref());
        }
    }

    pub fn get_device_list(&self) -> Vec<String> {
        self.device_list.iter().cloned().collect()
    }

    pub fn get_online_device_list(&self) -> Vec<String> {
        self.online_device_list.iter().cloned().collect()
    }

    pub async fn add_entry(&self, msg: ClipboardMessage, store: bool) -> anyhow::Result<()> {
        debug!("Publishing message: {:?}", msg);
        self.sender.send(msg.clone())?;
        if self.validate_message_content(&msg).await.is_err() {
            warn!("Ignored invalid clipboard entry.");
            return Ok(());
        }
        if matches!(msg.entry.content, ServerClipboardContent::Text(_)) {
            let search = self.search.clone();
            self.thread_pool
                .spawn_blocking(move || -> anyhow::Result<()> {
                    if store {
                        debug!("Store clipboard entry {:?}", msg);
                        match search.add_entry(&msg) {
                            Ok(_) => {}
                            Err(e) => {
                                warn!("Failed to store clipboard entry: {}", e);
                            }
                        }
                    }
                    Ok(())
                })
                .await??;
        }
        Ok(())
    }

    pub async fn query(&self, param: QueryParam) -> anyhow::Result<QueryResult> {
        let search = self.search.clone();
        self.thread_pool
            .spawn_blocking(move || -> anyhow::Result<QueryResult> { search.query(param) })
            .await?
    }

    async fn validate_message_content(&self, msg: &ClipboardMessage) -> anyhow::Result<()> {
        match &msg.entry.content {
            ServerClipboardContent::Text(s) => {
                if s.is_empty() {
                    anyhow::bail!("Empty clipboard entry, ignored.");
                }
            }
            ServerClipboardContent::ImageUrl(s) => {
                if s.is_empty() {
                    anyhow::bail!("Empty clipboard entry, ignored.");
                }

                // TODO: Test if the image exists.
            }
        }
        Ok(())
    }
}
