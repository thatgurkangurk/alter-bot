use std::path::Path;

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use alterbot::{bot::create_bot, config::ConfigManager, consts};
use anyhow::Result;

macro_rules! info { ($($arg:tt)*) => { tracing::info!(target: "alterbot::main", $($arg)*) }; }
macro_rules! warn { ($($arg:tt)*) => { tracing::warn!(target: "alterbot::main", $($arg)*) }; }
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,alterbot=info"));

    let timer = fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string());

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().compact().with_target(true).with_timer(timer))
        .init();

    print_startup_info();

    let config_manager = load_config_manager()?;

    config_manager.watch()?;

    let mut bot = create_bot(config_manager).await?;

    if let Err(why) = bot.start().await {
        error!("bot crashed: {:?}", why);
    }

    Ok(())
}
