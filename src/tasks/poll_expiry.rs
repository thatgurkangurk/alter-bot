use ::serenity::model::Colour;
use chrono::Utc;
use poise::serenity_prelude as serenity;
use sea_orm::{ActiveModelTrait, ColumnTrait, QueryFilter, Set, entity::prelude::*};
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

use crate::bot::PollCache;
use crate::models::{guild, poll, vote};
use crate::utils::renderer::generate_results_chart;

/// runs every second, checking the cache.
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
            // fetch the full poll from db to ensure it hasn't already been processed
            let Ok(Some(active_poll)) = poll::Entity::find_by_id(poll_id)
                .filter(poll::Column::IsActive.eq(true))
                .one(&db)
                .await
            else {
                // it doesn't exist or is already inactive, just clean the cache
                cache.write().await.remove(&poll_id);
                continue;
            };

            // mark inactive immediately to prevent double processing
            let mut am: poll::ActiveModel = active_poll.clone().into();
            am.is_active = Set(false);
            if am.update(&db).await.is_err() {
                continue;
            }

            // remove from cache
            cache.write().await.remove(&poll_id);

            // fetch votes, generate chart, and update discord
            let votes = vote::Entity::find()
                .filter(vote::Column::PollId.eq(poll_id))
                .all(&db)
                .await
                .unwrap_or_default();
            let vote_data: Vec<(i64, vote::VoteChoice)> =
                votes.into_iter().map(|v| (v.user_id, v.choice)).collect();

            let chart = generate_results_chart(&vote_data);

            let results_embed = serenity::CreateEmbed::new()
                .title(format!("poll closed: {}", active_poll.title))
                .description(&chart)
                .color(Colour::RED);

            if let Some(msg_id) = active_poll.message_id {
                let channel_id = serenity::ChannelId::new(active_poll.channel_id.cast_unsigned());
                let message_id = serenity::MessageId::new(msg_id.cast_unsigned());

                let builder = serenity::EditMessage::new()
                    .embed(results_embed)
                    .components(vec![]);

                let _ = channel_id.edit_message(&http, message_id, builder).await;
            }

            // admin logging
            if let Ok(Some(guild_config)) = guild::Entity::find_by_id(active_poll.guild_id)
                .one(&db)
                .await
                && let Some(log_channel_id) = guild_config.log_channel_id
            {
                let log_channel = serenity::ChannelId::new(log_channel_id.cast_unsigned());
                let guild_id = serenity::GuildId::new(active_poll.guild_id.cast_unsigned());

                let mut yes_votes = Vec::new();
                let mut no_votes = Vec::new();
                let mut hard_no_votes = Vec::new();

                for (user_id, choice) in vote_data {
                    match choice {
                        vote::VoteChoice::Yes => yes_votes.push(user_id),
                        vote::VoteChoice::No => no_votes.push(user_id),
                        vote::VoteChoice::HardNo => hard_no_votes.push(user_id),
                    }
                }

                let mut log_content = format!(
                    "**poll closed: {}**\n*the following votes were cast:*\n```text\n",
                    active_poll.title
                );

                let categories = [
                    ("yes", yes_votes),
                    ("no", no_votes),
                    ("hard no", hard_no_votes),
                ];

                for (i, (label, votes)) in categories.iter().enumerate() {
                    let _ = writeln!(log_content, "{label}");
                    let _ = writeln!(log_content, "----");

                    if votes.is_empty() {
                        let _ = writeln!(log_content, "no one voted for this");
                    } else {
                        for &id in votes {
                            let user_id = serenity::UserId::new(id.cast_unsigned());

                            // fetch the member object from Discord to get their server nickname
                            let display_name = guild_id.member(&http, user_id).await.map_or_else(
                                |_| format!("Unknown User ({id})"),
                                |member| member.display_name().to_owned(),
                            );

                            let _ = writeln!(log_content, "{display_name}");
                        }
                    }

                    if i < categories.len() - 1 {
                        let _ = writeln!(log_content);
                    }
                }

                let _ = write!(log_content, "```");

                let _ = log_channel
                    .send_message(&http, serenity::CreateMessage::new().content(log_content))
                    .await;
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
