mod cache;
mod commands;
mod events;
mod internal;
mod modal;
mod tasks;

use poise::serenity_prelude as serenity;
use std::sync::Arc;

pub use cache::PollCache;
pub use commands::poll_commands as commands;
pub use events::event_handler;
pub use internal::logic::{CreatePollParams, create_and_post_poll};

pub fn spawn_background_tasks(
    http: Arc<serenity::Http>,
    db: sea_orm::DatabaseConnection,
    cache: PollCache,
) {
    let fast_db = db.clone();
    let fast_cache = cache.clone();

    tokio::spawn(async move {
        tasks::run_fast_loop(http, fast_db, fast_cache).await;
    });

    tokio::spawn(async move {
        tasks::run_sync_loop(db, cache).await;
    });
}
