use std::{collections::HashSet, path::PathBuf};

use client_interface::{QueryParams, ServerClipboardContent};
use log::{debug, info, warn};
use moka::future::Cache;
use sha2::Digest;
use tokio::{
    io::AsyncReadExt,
    runtime::{Builder, Handle},
    sync::broadcast::Sender,
};

use super::{ClipboardMessage, QueryResult, ServerConfig};

pub struct GlobalState {
    sender: Sender<ClipboardMessage>,
    device_list: HashSet<String>,
    online_device_list: HashSet<String>,
    search: storage::Storage,
    _rt: tokio::runtime::Runtime,
    thread_pool: Handle,
    image_path: PathBuf,
    cache: Cache<String, String>,
}

impl GlobalState {
    pub fn new(args: &ServerConfig, sender: Sender<ClipboardMessage>) -> Self {
        let rt = Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("search-pool")
            .build()
            .unwrap();

        let handle = rt.handle().clone();

        let search = storage::Storage::new(args.index_path.clone());
        let device_list = search.get_device_list().unwrap();

        Self {
            sender,
            device_list,
            online_device_list: HashSet::new(),
            search,
            _rt: rt,
            thread_pool: handle,
            image_path: args.image_path.clone().unwrap(),
            cache: Cache::new(10_000),
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

    pub async fn add_entry(&self, mut msg: ClipboardMessage, store: bool) -> anyhow::Result<()> {
        debug!("Publishing message: {:?}", msg);
        if self.validate_message_content(&msg).await.is_err() {
            warn!("Ignored invalid clipboard entry.");
            return Ok(());
        }
        self.sender.send(msg.clone())?;
        match &msg.entry.content {
            ServerClipboardContent::ImageUrl(url) => {
                let digest = self.image_digest(url).await?;
                msg.entry.id = Some(digest);
            }
            ServerClipboardContent::Text(text) => {
                let mut hasher = <sha2::Sha512 as Digest>::new();
                hasher.update(text.as_bytes());
                let digest = hex::encode(std::convert::Into::<[u8; 64]>::into(hasher.finalize()));
                msg.entry.id = Some(digest);
            }
        }
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
        Ok(())
    }

    pub async fn query(&self, param: QueryParams) -> anyhow::Result<QueryResult> {
        let search = self.search.clone();
        let result = self
            .thread_pool
            .spawn_blocking(move || -> anyhow::Result<QueryResult> { search.query(param) })
            .await??;
        for msg in result.data.iter() {
            self.update_image_digest_cache(msg).await;
        }
        Ok(result)
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

                let digest = self.image_digest(s).await?;
                if digest.is_empty() {
                    anyhow::bail!("Image not found.");
                }
                self.cache.insert(s.to_string(), digest.to_owned()).await;
            }
        }
        Ok(())
    }

    pub async fn get_entry_by_id(&self, id: &str) -> anyhow::Result<Option<ClipboardMessage>> {
        let search = self.search.clone();
        let id = id.to_string();
        match self
            .thread_pool
            .spawn_blocking(move || search.get_entry_by_id(&id))
            .await?
        {
            Ok(Some(msg)) => {
                self.update_image_digest_cache(&msg).await;
                Ok(Some(msg))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn update_image_digest_cache(&self, msg: &ClipboardMessage) {
        // Whenever we do query, we update the image digest cache if possible.
        if let ServerClipboardContent::ImageUrl(url) = &msg.entry.content {
            self.cache
                .insert(
                    url.to_string(),
                    msg.entry
                        .id
                        .to_owned()
                        .expect("Internal error, index corrupted."),
                )
                .await;
        }
    }

    async fn image_digest(&self, url: &str) -> anyhow::Result<String> {
        let path = self.image_path.join(url);
        let path = path.to_str().unwrap();
        let digest = self
            .cache
            .get_with(path.to_string(), async move {
                let Ok(mut file) = tokio::fs::File::open(path).await else {
                    return Default::default();
                };
                let mut buf = Vec::with_capacity(4096);
                let mut read_bytes = 0;
                let mut hasher = <sha2::Sha512 as Digest>::new();
                while let Ok(n) = file.read_buf(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    read_bytes += n;
                    hasher.update(&buf[0..n]);
                    buf.clear();
                }
                if read_bytes == 0 {
                    return Default::default();
                }
                hex::encode(std::convert::Into::<[u8; 64]>::into(hasher.finalize()))
            })
            .await;
        if digest.is_empty() {
            anyhow::bail!("Image not found.");
        }
        Ok(digest)
    }
}
