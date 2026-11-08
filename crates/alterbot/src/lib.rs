mod cache;
pub mod consts;
mod context;
mod entity;
mod module;
mod module_manager;
mod util;

use crate::module_manager::MANAGER;
pub use context::CTX;

use anyhow::Context as _;
use sea_orm::Database;
use tokio::{runtime::Builder as RuntimeBuilder, signal};
use tracing::{Instrument as _, instrument::Instrumented};
use twilight_gateway::{
    ConfigBuilder, Event, EventTypeFlags, Intents, Shard, StreamExt, create_recommended,
};
use twilight_http::Client as HttpClient;

use alterconfig::ConfigManager;

const EVENT_TYPES: EventTypeFlags = EventTypeFlags::all();
const INTENTS: Intents = Intents::all();

pub fn start_bot(
    config_manager: ConfigManager,
    sync_commands: bool,
    exit_after_sync: bool,
) -> anyhow::Result<()> {
    let config = config_manager.get_sync();

    let rt = RuntimeBuilder::new_multi_thread().enable_all().build()?;

    rt.block_on(async move {
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

        tracing::info!("connecting to db...");
        let db = Database::connect(&config.db.uri).await?;

        let crate_name = module_path!().split("::").next().unwrap();
        let pattern = format!("{crate_name}::entity::*");

        tracing::info!("syncing all db schema using pattern {}", pattern);
        db.get_schema_registry(&pattern).sync(&db).await?;
        tracing::info!("db schema successfully synced");

        let gateway_config =
            ConfigBuilder::new(config.bot.token.as_ref().to_string(), INTENTS).build();

        let shards =
            create_recommended(&http, gateway_config, |_, builder| builder.build()).await?;

        context::init(app.id, http, config_manager, db);

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

        let runners: Vec<_> = shards
            .into_iter()
            .map(|shard| tokio::spawn(run_shard(shard)))
            .collect();

        signal::ctrl_c().await?;
        tracing::info!("shutting down; press CTRL-C to abort");

        for runner in runners {
            runner.abort();
        }

        anyhow::Ok(())
    })
}

async fn run_shard(mut shard: Shard) {
    while let Some(event) = shard.next_event(EVENT_TYPES).await {
        match event {
            Ok(event) => {
                CTX.cache.update(&event);

                tokio::spawn(async move {
                    event_handler(event).await;
                });
            }
            Err(error) => {
                tracing::warn!(shard = %shard.id(), %error, "Error receiving gateway event");
            }
        }
    }
}

async fn event_handler(event: Event) {
    async fn log_err(future: Instrumented<impl std::future::Future<Output = anyhow::Result<()>>>) {
        let mut future = std::pin::pin!(future);
        if let Err(error) = future.as_mut().await {
            let _enter = future.span().enter();
            tracing::warn!(error = &*error, "failed to handle event");
        }
    }

    match event {
        Event::Ready(ready) => {
            let name: String = ready
                .user
                .global_name
                .clone()
                .unwrap_or_else(|| ready.user.name.clone());

            tracing::info!(name = %name, "shard ready");
        }

        Event::InteractionCreate(interaction) => {
            let span = tracing::info_span!("interaction", id = %interaction.id);
            log_err(MANAGER.handle_interaction(interaction).instrument(span)).await;
        }

        other_event => {
            MANAGER.handle_event(&other_event).await;
        }
    }
}
