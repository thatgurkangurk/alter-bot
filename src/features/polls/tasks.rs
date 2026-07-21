use chrono::Utc;
use poise::serenity_prelude as serenity;
use sea_orm::{ColumnTrait, QueryFilter, entity::prelude::*};
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

use super::cache::PollCache;
use crate::models::poll;

/// runs every second, checking the cache for expired polls and finalising them.
#[allow(clippy::too_many_lines)]
pub async fn run_fast_loop(
    http: Arc<serenity::Http>,
    db: sea_orm::DatabaseConnection,
    cache: PollCache,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        // Drain/extract expired poll ids
        let expired_ids = cache.retain_expired();

        if expired_ids.is_empty() {
            continue;
        }

        for poll_id in expired_ids {
            let poll_opt = poll::Entity::find_by_id(poll_id)
                .filter(poll::Column::IsActive.eq(true))
                .one(&db)
                .await;

            if let Ok(Some(active_poll)) = poll_opt {
                if let Err(e) =
                    super::internal::logic::close_and_finalize_poll(&http, &db, &cache, active_poll)
                        .await
                {
                    error!("Error finalizing poll {poll_id}: {e}");
                }
            } else {
                // if it's no longer active in the DB, make sure it's removed from cache
                cache.remove(&poll_id);
            }
        }
    }
}

/// runs every 30 seconds, making sure cache equals db state.
pub async fn run_sync_loop(db: sea_orm::DatabaseConnection, cache: PollCache) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        let active_polls = match poll::Entity::find()
            .filter(poll::Column::IsActive.eq(true))
            .all(&db)
            .await
        {
            Ok(polls) => polls,
            Err(e) => {
                error!("Cache sync DB error: {e:?}");
                continue;
            }
        };

        // convert query results into iterator of (Uuid, DateTime<Utc>)
        let fresh_polls = active_polls
            .into_iter()
            .map(|p| (p.id, p.ends_at.with_timezone(&Utc)));

        // replaces cache contents and evicts stale/inactive entries
        cache.sync_all(fresh_polls);
    }
}
