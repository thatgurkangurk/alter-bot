use std::path::Path;

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use alterbot::{consts, start_bot};
use alterconfig::ConfigManager;
use anyhow::Result;

macro_rules! info { ($($arg:tt)*) => { tracing::info!(target: "alterbot::main", $($arg)*) }; }
macro_rules! warn { ($($arg:tt)*) => { tracing::warn!(target: "alterbot::main", $($arg)*) }; }
#[allow(unused)]
macro_rules! error { ($($arg:tt)*) => { tracing::error!(target: "alterbot::main", $($arg)*) }; }

fn print_startup_info() {
    info!("==========================================");
    info!("alter-bot version {} by gurkan", consts::VERSION);
    info!("MPL 2.0 license");
    info!("{}", &consts::REPOSITORY);
    info!("==========================================");
}

fn load_config_manager() -> Result<ConfigManager> {
    let default_path = Path::new(consts::DATA_DIR).join("alter-bot.toml");

    if !default_path.exists() {
        warn!("{} does not exist", default_path.display());

        if std::env::var("CREATE_CONFIG_FILE_IF_NOT_EXIST").unwrap_or_default() == "1" {
            warn!(
                "creating an empty config file at: {}",
                default_path.display()
            );

            if let Some(parent) = default_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&default_path, "")?;
        }
    }

    ConfigManager::new(Some(&default_path))
}

fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls ring crypto provider");

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,alterbot=debug"));

    let timer = fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string());

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().compact().with_target(true).with_timer(timer))
        .init();

    print_startup_info();

    let config_manager = load_config_manager()?;

    config_manager.watch()?;

    let config = config_manager.get_sync();

    start_bot(&config)?;

    Ok(())
}
