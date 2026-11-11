mod cache;
pub mod consts;
mod context;
mod emoji;
mod entity;
mod module;
mod module_manager;
mod util;
mod web;

use alterconfig::ConfigManager;
use anyhow::Context as _;
use reqwest::ClientBuilder;
use tokio::signal;
use twilight_http::Client as HttpClient;

use crate::module_manager::MANAGER;
pub use context::CTX;

pub async fn start_bot(
    config_manager: ConfigManager,
    sync_commands: bool,
    exit_after_sync: bool,
) -> anyhow::Result<()> {
    let config = config_manager.get_sync();

    let http = HttpClient::builder()
        .token(config.bot.token.as_ref().to_string())
        .build();

    let (user, app) = tokio::try_join!(
        async {
            http.current_user()
                .await?
                .model()
                .await
                .map_err(anyhow::Error::from)
        },
        async {
            http.current_user_application()
                .await?
                .model()
                .await
                .map_err(anyhow::Error::from)
        },
    )?;

    tracing::info!(user = %user.name, "logged in successfully");

    let http_client = ClientBuilder::new()
        .build()
        .context("failed to build reqwest client")?;

    context::init_early(app.id, http, config_manager, http_client);

    if sync_commands {
        tracing::info!("syncing application commands...");
        MANAGER
            .register_commands()
            .await
            .context("registering commands")?;
        tracing::info!("application commands successfully synced");
    }

    if exit_after_sync {
        tracing::info!("exit requested after sync operations; exiting");
        return anyhow::Ok(());
    }

    tracing::info!("initialising modules...");
    MANAGER.init().await.context("running module init steps")?;

    signal::ctrl_c().await?;
    tracing::info!("shutting down...");

    anyhow::Ok(())
}
