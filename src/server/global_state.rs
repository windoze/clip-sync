use std::collections::HashSet;

use log::{debug, info, warn};
use tokio::{
    runtime::{Builder, Handle},
    sync::broadcast::Sender,
};

use super::{search::Search, ClipboardData, QueryParam, ServerConfig};

pub struct GlobalState {
    sender: Sender<String>,
    device_list: HashSet<String>,
    online_device_list: HashSet<String>,
    search: Search,
    _rt: tokio::runtime::Runtime,
    thread_pool: Handle,
}

impl GlobalState {
    pub fn new(args: &ServerConfig, sender: Sender<String>) -> Self {
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
        }
    }

    pub fn get_receiver(&self) -> tokio::sync::broadcast::Receiver<String> {
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

    pub async fn add_entry(&self, entry: ClipboardData, store: bool) -> anyhow::Result<()> {
        debug!("Publishing message: {:?}", entry);
        self.sender.send(serde_json::to_string(&entry).unwrap())?;
        let search = self.search.clone();
        self.thread_pool
            .spawn_blocking(move || -> anyhow::Result<()> {
                if store {
                    debug!("Store clipboard entry {:?}", entry);
                    match search.add_entry(&entry) {
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

    pub async fn query(&self, param: QueryParam) -> anyhow::Result<Vec<ClipboardData>> {
        let search = self.search.clone();
        self.thread_pool
            .spawn_blocking(move || -> anyhow::Result<Vec<ClipboardData>> { search.query(param) })
            .await?
    }
}
