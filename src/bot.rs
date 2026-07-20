use ::serenity::Client;
use chrono::{DateTime, Utc};
use poise::serenity_prelude as serenity;
use sea_orm::DatabaseConnection;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::{commands, config::Config, db::create_db, features};

pub type PollCache = Arc<RwLock<HashMap<Uuid, DateTime<Utc>>>>;
pub struct Data {
    pub db: DatabaseConnection,
    pub cache: PollCache,
}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command)]
async fn info(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("i am alter bot").await?;
    Ok(())
}

#[allow(clippy::unused_async, clippy::single_match)]
async fn event_handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            info!("Logged in as {}", data_about_bot.user.name);
        }
        _ => {}
    }

    Ok(())
}

pub async fn create_bot(config: &Config) -> anyhow::Result<Client> {
    let intents = serenity::GatewayIntents::GUILDS;

    let db = create_db(config).await?;

    // this is a block to prevent someone (me) being stupid and modifying the vec outside this scope
    let commands = {
        // if a module has 2 or more commands, use cmds.extend instead
        let mut cmds = vec![
            info(),
            commands::minecraft::server_status(),
            commands::awty::are_we_there_yet(),
        ];
        cmds.extend(commands::settings::settings_commands());
        cmds.extend(features::polls::commands());
        cmds
    };

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands,
            event_handler: |ctx, event, framework, data| {
                Box::pin(async move {
                    event_handler(ctx, event, framework, data).await?;
                    crate::events::component::handle(ctx, event, framework, data).await?;
                    crate::awty::event::handle_persistent_buttons(ctx, event).await
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            let http_clone = std::sync::Arc::clone(&ctx.http);
            let db_clone_fast = db.clone();
            let db_clone_sync = db.clone();

            let cache = Arc::new(RwLock::new(HashMap::new()));
            let cache_clone_fast = Arc::clone(&cache);
            let cache_clone_sync = Arc::clone(&cache);

            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                tokio::spawn(async move {
                    features::polls::run_fast_loop(http_clone, db_clone_fast, cache_clone_fast)
                        .await;
                });

                tokio::spawn(async move {
                    features::polls::run_sync_loop(db_clone_sync, cache_clone_sync).await;
                });

                Ok(Data { db, cache })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(&config.bot.token, intents)
        .framework(framework)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create client: {e}"))?;

    Ok(client)
}
