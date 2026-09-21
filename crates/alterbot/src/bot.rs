use std::net::SocketAddr;
use std::sync::Arc;

use poise::{CreateReply, serenity_prelude as serenity};
use sea_orm::DatabaseConnection;
use serenity::Client;
use tracing::{info, warn};

use crate::{
    config::ConfigManager,
    consts,
    db::create_db,
    features::{self, polls::PollCache},
    web::WebServer,
};

pub struct Data {
    pub db: DatabaseConnection,
    pub cache: PollCache,
    pub config_manager: ConfigManager,
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
    if let serenity::FullEvent::Ready { data_about_bot, .. } = event {
        info!("Logged in as {}", data_about_bot.user.name);
    }

    Ok(())
}

pub async fn create_bot(config_manager: ConfigManager) -> anyhow::Result<Client> {
    let intents = serenity::GatewayIntents::all();

    let db = create_db(&config_manager).await?;
    let config = config_manager.get().await;

    let commands = vec![info(), features::awty::are_we_there_yet()];
    let commands = features::polls::commands(commands);
    let commands = features::settings::commands(commands);
    let commands = features::minecraft::commands(commands);
    let commands = features::quote::commands(commands);

    let poll_cache = PollCache::new();
    let is_dev = cfg!(debug_assertions);
    let dev_guild_id = config.bot.dev_guild_id;

    let setup_db = db.clone();
    let setup_poll_cache = poll_cache.clone();
    let setup_config_manager = config_manager.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands,
            event_handler: |ctx, event, framework, data| {
                Box::pin(async move {
                    event_handler(ctx, event, framework, data).await?;
                    features::polls::event_handler(ctx, event, framework, data).await?;
                    features::awty::handle_persistent_buttons(ctx, event).await?;
                    features::fun::event_handler(ctx, event, framework, data).await?;
                    features::anti_self_true::event_handler(ctx, event, framework, data).await
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            let http_clone = Arc::clone(&ctx.http);
            let db_for_tasks = setup_db.clone();
            let db_for_data = setup_db.clone();

            let cache_for_tasks = setup_poll_cache.clone();
            let cache_for_data = setup_poll_cache.clone();

            let config_manager_for_data = setup_config_manager.clone();

            Box::pin(async move {
                match (is_dev, dev_guild_id) {
                    (true, Some(guild_id)) => {
                        poise::builtins::register_in_guild(
                            ctx,
                            &framework.options().commands,
                            guild_id,
                        )
                        .await?;
                        info!("dev mode: registered commands in dev guild ({guild_id:?})");
                    }
                    (true, None) => {
                        warn!("dev mode active, but `dev_guild_id` is missing in config! skipping guild registration");
                    }
                    (false, _) => {
                        poise::builtins::register_globally(
                            ctx,
                            &framework.options().commands,
                        )
                        .await?;
                        info!("registered commands globally");
                    }
                }

                features::polls::spawn_background_tasks(http_clone, db_for_tasks, cache_for_tasks);

                Ok(Data {
                    db: db_for_data,
                    cache: cache_for_data,
                    config_manager: config_manager_for_data,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(config.bot.token, intents)
        .framework(framework)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create client: {e}"))?;

    let host = match option_env!("IS_IN_CONTAINER") {
        Some("1") => [0, 0, 0, 0],
        _ => [127, 0, 0, 1],
    };

    WebServer::builder()
        .http(client.http.clone())
        .shard_manager(client.shard_manager.clone())
        .config_manager(config_manager)
        .poll_cache(poll_cache)
        .db(db)
        .bind(SocketAddr::from((host, config.web.port.unwrap_or(3000))))
        .build()?
        .run();

    Ok(client)
}
