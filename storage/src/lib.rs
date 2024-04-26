use client_interface::{ClipboardMessage, QueryParams, QueryResult};
use std::collections::HashSet;

mod tantivy_storage;
pub type Storage = tantivy_storage::Storage;

mod db_storage;

pub trait ClipStorage {
    fn add_entry(
        &self,
        entry: &ClipboardMessage,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;

    fn get_entry_by_id(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<ClipboardMessage>>> + Send;

    fn get_device_list(
        &self,
    ) -> impl std::future::Future<Output = anyhow::Result<HashSet<String>>> + Send;

    fn query(
        &self,
        params: QueryParams,
    ) -> impl std::future::Future<Output = anyhow::Result<QueryResult>> + Send;
}

pub fn open_storage<'a>(storage_location: impl Into<Option<&'a str>>) -> Storage {
    Storage::new(storage_location.into())
}
