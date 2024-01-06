use std::{collections::HashSet, path::PathBuf};

use chrono::{DateTime, TimeZone, Utc};
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

fn default_timestamp() -> i64 {
    Utc::now().timestamp()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardMessage {
    #[serde(flatten)]
    pub entry: ServerClipboardRecord,
    #[serde(default = "default_timestamp")]
    pub timestamp: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerClipboardRecord {
    pub source: String,
    #[serde(flatten)]
    pub content: ServerClipboardContent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerClipboardContent {
    Text(String),
    ImageUrl(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Params {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub begin: Option<i64>,
    #[serde(default)]
    pub end: Option<i64>,
    #[serde(default)]
    pub size: Option<usize>,
    #[serde(default)]
    pub skip: Option<usize>,
    #[serde(default)]
    pub sort: Option<String>,
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

pub struct QueryParam {
    pub query: Option<String>,
    pub sources: HashSet<String>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub skip: usize,
    pub size: usize,
    pub sort_by_score: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub total: usize,
    pub skip: usize,
    pub data: Vec<ClipboardMessage>,
}
