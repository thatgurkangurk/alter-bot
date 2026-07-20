use ::serenity::Client;
use chrono::{DateTime, Utc};
use poise::{CreateReply, serenity_prelude as serenity};
use sea_orm::DatabaseConnection;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::{config::Config, consts, db::create_db, features};

pub type PollCache = Arc<RwLock<HashMap<Uuid, DateTime<Utc>>>>;
pub struct Data {
    pub db: DatabaseConnection,
    pub cache: PollCache,
}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
pub type Command = poise::Command<Data, Error>;

#[poise::command(slash_command)]
async fn info(ctx: Context<'_>) -> Result<(), Error> {
    let bot_user = ctx.cache().current_user().clone();

    let embed = serenity::CreateEmbed::new()
        .title("alter bot")
        .field("version", consts::VERSION, true)
        .field("authors", consts::AUTHORS_RAW.replace(':', ", "), true)
        .field("repository", consts::REPOSITORY, false)
        .colour(serenity::Colour::from_rgb(236, 253, 245))
        .timestamp(serenity::Timestamp::now())
        .thumbnail(bot_user.face());

    ctx.send(CreateReply::default().embed(embed)).await?;

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

    let commands = vec![info(), features::awty::are_we_there_yet()];

    let commands = features::polls::commands(commands);
    let commands = features::settings::commands(commands);
    let commands = features::minecraft::commands(commands);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands,
            event_handler: |ctx, event, framework, data| {
                Box::pin(async move {
                    event_handler(ctx, event, framework, data).await?;
                    features::polls::event_handler(ctx, event, framework, data).await?;
                    features::awty::handle_persistent_buttons(ctx, event).await
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
