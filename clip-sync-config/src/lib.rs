use std::path::{Path, PathBuf};

use clap::Parser;
use platform_dirs::AppDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Args {
    pub roles: Vec<String>,
    #[cfg(feature = "server")]
    #[serde(default)]
    pub server: websocket_server::ServerConfig,
    #[cfg(feature = "mqtt")]
    #[serde(default)]
    pub mqtt_client: mqtt_client::MqttClientConfig,
    #[cfg(feature = "websocket")]
    #[serde(default)]
    pub websocket_client: websocket_client::ClientConfig,
}

impl Args {
    #[cfg(feature = "websocket")]
    pub fn get_server_url(&self) -> Option<String> {
        if self.roles.contains(&"websocket-client".to_string()) {
            if let Ok(mut url) = url::Url::parse(&self.websocket_client.server_url) {
                let scheme = if (url.scheme() == "wss") || url.scheme() == "https" {
                    "https"
                } else if (url.scheme() == "ws") || url.scheme() == "http" {
                    "http"
                } else {
                    return None;
                };
                url.set_scheme(scheme).unwrap();
                url.set_path("");
                Some(url.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    #[cfg(not(feature = "websocket"))]
    #[allow(dead_code, unused_variables)]
    pub fn get_server_url(&self) -> Option<String> {
        None
    }
}

fn get_config_file() -> PathBuf {
    let app_dirs = AppDirs::new(Some("clip-sync"), false).unwrap();
    app_dirs.config_dir.join("config.toml")
}

pub fn parse_config<P: AsRef<Path>>(config_path: Option<P>) -> anyhow::Result<Args> {
    let config_path: PathBuf = config_path
        .map(|p| p.as_ref().to_path_buf())
        .unwrap_or(get_config_file());
    let config = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Failed to read config at '{:?}'", config_path));
    Ok(toml::from_str::<Args>(&config)?)
}

pub fn parse() -> anyhow::Result<Args> {
    #[derive(Debug, Clone, Parser)]
    struct Config {
        #[arg(long = "config")]
        config_path: Option<std::path::PathBuf>,
        #[arg(long, default_value = "false")]
        no_tray: bool,
        #[command(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    }

    let cli = Config::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .filter_module("tantivy", log::LevelFilter::Warn) // Tantivy is too talky at the INFO level
        .init();
    parse_config(cli.config_path)
}
