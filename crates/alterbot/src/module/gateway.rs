use anyhow::Context;
use std::future::Future;
use tracing::Instrument;
use twilight_gateway::{ConfigBuilder, Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt};

use crate::{
    module::{Module, ModuleEntry},
    module_manager::MANAGER,
};

pub struct GatewayModule;

const EVENT_TYPES: EventTypeFlags = EventTypeFlags::all();
const INTENTS: Intents = Intents::all();

#[async_trait::async_trait]
impl Module for GatewayModule {
    fn name(&self) -> &'static str {
        "gateway"
    }

    async fn init(&self) -> anyhow::Result<()> {
        let http = crate::context::http();
        let config = crate::context::config();

        let gateway_config =
            ConfigBuilder::new(config.bot.token.as_ref().to_string(), INTENTS).build();

        let gateway_info = http
            .gateway()
            .authed()
            .await
            .context("failed to fetch gateway info")?
            .model()
            .await
            .context("failed to deserialize gateway info")?;

        let total_shards = gateway_info.shards;

        for shard_index in 0..total_shards {
            let shard_id = ShardId::new(shard_index, total_shards);
            let shard = Shard::with_config(shard_id, gateway_config.clone());

            tokio::spawn(async move {
                if let Err(err) = run_shard(shard).await {
                    tracing::error!(shard_id = %shard_index, error = %err, "shard exited with error");
                }
            });
        }

        tracing::info!(total_shards, "spawned all gateway shards");
        Ok(())
    }
}

inventory::submit! {
    ModuleEntry {
        module: &GatewayModule
    }
}

async fn run_shard(mut shard: Shard) -> anyhow::Result<()> {
    while let Some(event) = shard.next_event(EVENT_TYPES).await {
        match event {
            Ok(event) => {
                crate::context::cache().update(&event);

                tokio::spawn(async move {
                    event_handler(event).await;
                });
            }
            Err(error) => {
                tracing::warn!(shard = %shard.id(), %error, "error receiving gateway event");
            }
        }
    }

    anyhow::bail!("shard event stream terminated unexpectedly")
}

async fn event_handler(event: Event) {
    match event {
        Event::Ready(ready) => {
            let name = ready
                .user
                .global_name
                .clone()
                .unwrap_or_else(|| ready.user.name.clone());

            tracing::info!(name = %name, "shard ready");
        }

        Event::InteractionCreate(interaction) => {
            let span = tracing::info_span!("interaction", id = %interaction.id);

            MANAGER
                .handle_event(&Event::InteractionCreate(interaction.clone()))
                .await;

            log_err(MANAGER.handle_interaction(interaction).instrument(span)).await;
        }

        other_event => {
            MANAGER.handle_event(&other_event).await;
        }
    }
}

async fn log_err<F>(future: F)
where
    F: Future<Output = anyhow::Result<()>>,
{
    if let Err(error) = future.await {
        tracing::warn!(error = %error, "failed to handle interaction");
    }
}
