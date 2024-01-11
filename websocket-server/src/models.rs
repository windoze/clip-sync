use std::{collections::HashSet, path::PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use client_interface::{ClipboardMessage, Params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ServerConfig {
    pub endpoint: String,
    pub secret: Option<String>,
    #[serde(default)]
    pub use_tls: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub web_root: Option<PathBuf>,
    pub index_path: Option<PathBuf>,
    pub image_path: Option<PathBuf>,
}

pub struct QueryParam {
    pub query: Option<String>,
    pub sources: HashSet<String>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub skip: usize,
    pub size: usize,
    pub sort_by_score: bool,
}

impl From<Params> for QueryParam {
    fn from(val: Params) -> Self {
        QueryParam {
            query: val.q,
            sources: val
                .from
                .unwrap_or_default()
                .split(',')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            time_range: match (val.begin, val.end) {
                (Some(begin), Some(end)) => Some((
                    Utc.timestamp_opt(begin, 0).unwrap(),
                    Utc.timestamp_opt(end, 0).unwrap(),
                )),
                (Some(begin), None) => Some((Utc.timestamp_opt(begin, 0).unwrap(), Utc::now())),
                (None, Some(end)) => Some((
                    Utc.timestamp_opt(0, 0).unwrap(),
                    Utc.timestamp_opt(end, 0).unwrap(),
                )),
                _ => None,
            },
            skip: val.skip.unwrap_or(0),
            size: val.size.unwrap_or(10),
            sort_by_score: val.sort.unwrap_or("time".to_string()) == "score",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub total: usize,
    pub skip: usize,
    pub data: Vec<ClipboardMessage>,
}
