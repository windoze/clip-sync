use std::path::PathBuf;

use clap::Parser;
use log::info;
use platform_dirs::AppDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Args {
    #[serde(default)]
    pub server: websocket_server::ServerConfig,
}

fn get_config_file() -> PathBuf {
    let app_dirs = AppDirs::new(Some("clip-sync"), false).unwrap();
    app_dirs.config_dir.join("config.toml")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[derive(Debug, Clone, Parser)]
    struct Config {
        #[arg(long = "config")]
        config_path: Option<PathBuf>,
        #[cfg(not(feature = "server-only"))]
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
    let config_path = cli.config_path.unwrap_or(get_config_file());
    let config = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Failed to read config at '{:?}'", config_path));
    let args = toml::from_str::<Args>(&config)?;

    info!("Starting websocket server");
    websocket_server::server_main(args.server)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}
