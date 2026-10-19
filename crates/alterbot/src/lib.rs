pub mod consts;
mod context;
mod dispatch;
mod module;
mod module_manager;
mod resume;

use crate::module_manager::MANAGER;

pub use self::{
    context::CTX,
    dispatch::{ShardHandle, ShardRestartKind},
    resume::{ConfigBuilderExt, Info as ResumeInfo},
};

use anyhow::Context as _;
use std::{convert::identity, pin::pin, time::Duration};
use tokio::{runtime::Builder as RuntimeBuilder, signal};
use tokio_stream::StreamExt as _;
use tracing::{Instrument as _, instrument::Instrumented};
use twilight_gateway::{
    ConfigBuilder, Event, EventTypeFlags, Intents, Shard, queue::InMemoryQueue,
};
use twilight_http::Client;

use alterconfig::Config;

const EVENT_TYPES: EventTypeFlags = EventTypeFlags::all();
const INTENTS: Intents = Intents::all();

pub fn start_bot(config: &Config) -> anyhow::Result<()> {
    let rt = RuntimeBuilder::new_current_thread().enable_all().build()?;
    let _guard = rt.enter();

    let http = Client::new(config.bot.token.as_ref().to_string());

    let app = rt
        .block_on(async { anyhow::Ok(http.current_user_application().await?.model().await?) })
        .context("getting app")?;
    let info = rt
        .block_on(async { anyhow::Ok(http.gateway().authed().await?.model().await?) })
        .context("getting info")?;

    let queue = InMemoryQueue::new(
        info.session_start_limit.max_concurrency,
        info.session_start_limit.remaining,
        Duration::from_millis(info.session_start_limit.reset_after),
        info.session_start_limit.total,
    );

    let bot_config = ConfigBuilder::new(config.bot.token.as_ref().to_string(), INTENTS)
        .queue(queue)
        .build();

    let shards = resume::restore(bot_config, info.shards);

    context::init(app.id, http, shards.len() as u32);

    let resume_info = rt.block_on(event_loop(shards))?;

    resume::save(&resume_info).context("saving resume info")?;

    Ok(())
}

async fn event_loop(shards: impl Iterator<Item = Shard>) -> anyhow::Result<Box<[ResumeInfo]>> {
    MANAGER
        .register_commands()
        .await
        .context("registering commands")?;

    let tasks = shards
        .map(|shard| tokio::spawn(dispatch::run(event_handler, shard, |_shard| ())))
        .collect::<Box<_>>();

    signal::ctrl_c().await?;
    tracing::info!("shutting down; press CTRL-C to abort");

    let results = tokio_stream::iter(tasks).then(identity);
    tokio::select! {
        _ = signal::ctrl_c() => Ok(Box::default()),
        resume_info = results.collect::<Result<_, _>>() => anyhow::Ok(resume_info?),
    }
}

async fn event_handler(event: Event, _state: ()) {
    async fn log_err(future: Instrumented<impl Future<Output = anyhow::Result<()>>>) {
        let mut future = pin!(future);
        if let Err(error) = future.as_mut().await {
            let _enter = future.span().enter();
            tracing::warn!(error = &*error, "failed to handle event");
        }
    }

    match event {
        Event::InteractionCreate(interaction) => {
            let span = tracing::info_span!("interaction", id = %interaction.id);
            log_err(MANAGER.handle_interaction(interaction).instrument(span)).await;
        }

        other_event => {
            MANAGER.handle_event(&other_event).await;
        }
    }
}
