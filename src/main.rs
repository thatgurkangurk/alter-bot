use std::path::Path;

use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{bot::create_bot, config::Config};

mod awty;
mod bot;
mod commands;
mod config;
mod consts;
mod db;
mod emojis;
mod events;
mod models;
mod tasks;
mod util;
mod utils;

fn print_startup_info() {
    info!("alter-bot version {} by gurkan", consts::VERSION);
    info!("MPL 2.0 license");
    info!("{}", &consts::REPOSITORY);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,alter_bot=info"));

    let timer = fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string());

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().compact().with_target(true).with_timer(timer))
        .init();

    print_startup_info();

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

    let config = Config::load(Some(&default_path))?;

    let mut bot = create_bot(&config).await?;

    if let Err(why) = bot.start().await {
        tracing::error!("bot crashed: {:?}", why);
    }

    Ok(())
}
