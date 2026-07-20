use chrono::Utc;
use poise::serenity_prelude as serenity;
use sea_orm::{ColumnTrait, QueryFilter, entity::prelude::*};
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

use crate::bot::PollCache;
use crate::models::poll;

/// runs every second, checking the cache.
#[allow(clippy::too_many_lines)]
pub async fn run_fast_loop(
    http: Arc<serenity::Http>,
    db: sea_orm::DatabaseConnection,
    cache: PollCache,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        let mut expired_ids = Vec::new();

        // scope the read lock so we don't block command insertions
        {
            let cache_read = cache.read().await;
            let now = Utc::now();
            for (&poll_id, &ends_at) in cache_read.iter() {
                if ends_at <= now {
                    expired_ids.push(poll_id);
                }
            }
        }

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
                    eprintln!("Error finalising poll {poll_id}: {e}");
                }
            } else {
                cache.write().await.remove(&poll_id);
            }
        }
    }
}

/// runs every 30 seconds, makes sure cache equals db
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

        let mut cache_write = cache.write().await;

        for p in active_polls {
            let ends_at_utc = p.ends_at.with_timezone(&Utc);
            cache_write.insert(p.id, ends_at_utc);
        }
    }
}
