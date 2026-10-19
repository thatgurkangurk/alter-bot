use std::collections::HashMap;
use std::fmt::Write;
use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use chrono::{Duration, Utc};
use poise::serenity_prelude as serenity;
use sea_orm::{ActiveModelTrait, QueryFilter, Set, entity::prelude::*};
use serde::Deserialize;

use super::renderer::generate_results_chart;
use crate::features::polls::cache::PollCache;
use crate::features::polls::internal::embed::build_poll_embed;
use crate::models::{guild, poll, poll_option, vote};

use tracing::{error, info, instrument};

#[derive(Deserialize)]
pub struct PollChoice {
    pub text: String,
    pub weight: Option<f64>,
}

#[allow(dead_code)]
impl PollChoice {
    pub fn new(text: impl Into<String>, weight: Option<f64>) -> Self {
        Self {
            text: text.into(),
            weight,
        }
    }

    /// parses a raw weight string (handling commas like "1,5" -> 1.5)
    pub fn parse_weight(input: Option<&str>) -> f64 {
        input
            .and_then(|s| s.trim().replace(',', ".").parse::<f64>().ok())
            .unwrap_or(1.0)
    }

    /// returns the parsed weight or defaults to 1.0
    pub fn weight_or_default(&self) -> f64 {
        self.weight.unwrap_or(1.0)
    }
}

impl FromStr for PollChoice {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(());
        }

        // split on the first colon (supports colons inside option labels)
        let mut parts = trimmed.splitn(2, ':');
        let text = parts.next().unwrap_or("").trim().to_string();

        if text.is_empty() {
            return Err(());
        }

        let weight = parts.next().map(|w| Self::parse_weight(Some(w)));

        Ok(Self { text, weight })
    }
}

pub struct CreatePollParams {
    pub title: String,
    pub guild_id: serenity::GuildId,
    pub target_channel_id: serenity::ChannelId,
    pub duration_minutes: i64,
    pub required_role_id: Option<serenity::RoleId>,
    pub choices: Vec<PollChoice>,
}

/// creates a new poll and sends a message in the discord channel
#[instrument(
    skip(db, http, params),
    fields(
        poll_id,
        guild_id = params.guild_id.get(),
        channel_id = params.target_channel_id.get(),
        title = %params.title
    )
)]
#[allow(clippy::too_many_lines)]
pub async fn create_and_post_poll(
    db: &DatabaseConnection,
    http: impl serenity::CacheHttp,
    params: CreatePollParams,
) -> Result<poll::Model> {
    let ends_at = Utc::now() + Duration::minutes(params.duration_minutes);
    let poll_id = Uuid::new_v4();

    tracing::Span::current().record("poll_id", poll_id.to_string());

    let mut poll_opts = Vec::new();
    let mut buttons = Vec::new();
    let mut labels = Vec::new();

    for (index, choice) in params.choices.into_iter().enumerate() {
        let label = choice.text.trim().to_string();
        if label.is_empty() {
            continue;
        }

        let weight = choice.weight.unwrap_or(1.0);

        let index_u32 = u32::try_from(index).unwrap_or(0);
        let emoji = char::from_u32(0x1F1E6 + index_u32);

        let opt_id = Uuid::new_v4();
        poll_opts.push(poll_option::ActiveModel {
            id: Set(opt_id),
            poll_id: Set(poll_id),
            label: Set(label.clone()),
            weight: Set(weight),
        });

        labels.push(label.clone());

        let mut button = serenity::CreateButton::new(format!("vote_{opt_id}_{poll_id}"))
            .label(label)
            .style(serenity::ButtonStyle::Secondary);

        if let Some(e) = emoji {
            button = button.emoji(e);
        }

        buttons.push(button);
    }

    if poll_opts.is_empty() {
        let err_msg = "a poll must have at least one valid option";
        error!(err_msg);
        return Err(anyhow!(err_msg));
    }

    let new_poll = poll::ActiveModel {
        id: Set(poll_id),
        guild_id: Set(params.guild_id.get().cast_signed()),
        channel_id: Set(params.target_channel_id.get().cast_signed()),
        title: Set(params.title.clone()),
        ends_at: Set(ends_at.fixed_offset()),
        is_active: Set(true),
        required_role_id: Set(params.required_role_id.map(|r| r.get().cast_signed())),
        ..Default::default()
    };

    let poll_model = new_poll
        .insert(db)
        .await
        .inspect_err(|e| {
            error!(
                error = ?e,
                "failed to insert primary poll record into database"
            );
        })
        .context("failed to insert poll into database")?;

    let option_count = poll_opts.len();
    poll_option::Entity::insert_many(poll_opts)
        .exec(db)
        .await
        .inspect_err(|e| {
            error!(
                error = ?e,
                option_count,
                "failed to insert poll options into database"
            );
        })
        .context("failed to insert poll options into database")?;

    let action_rows: Vec<serenity::CreateActionRow> = buttons
        .chunks(5)
        .map(|chunk| serenity::CreateActionRow::Buttons(chunk.to_vec()))
        .collect();

    let embed = build_poll_embed(
        &poll_model.title,
        ends_at,
        0,
        &labels,
        params.required_role_id,
    );

    let msg = params
        .target_channel_id
        .send_message(
            http,
            serenity::CreateMessage::new()
                .embed(embed)
                .components(action_rows),
        )
        .await
        .inspect_err(|e| {
            error!(
                error = ?e,
                "failed to post poll message to discord channel"
            );
        })
        .context("failed to send poll message to channel")?;

    let mut poll_am: poll::ActiveModel = poll_model.into();
    poll_am.message_id = Set(Some(msg.id.get().cast_signed()));

    let updated_poll = poll_am
        .update(db)
        .await
        .inspect_err(|e| {
            error!(
                error = ?e,
                message_id = msg.id.get(),
                "failed to update poll record with discord message ID"
            );
        })
        .context("failed to update poll with message ID")?;

    info!("poll successfully created and posted to discord");
    Ok(updated_poll)
}

#[tracing::instrument(
    skip(http, db, cache, poll_model),
    fields(
        poll_id = %poll_model.id,
        guild_id = poll_model.guild_id,
        channel_id = poll_model.channel_id,
        message_id = ?poll_model.message_id,
        title = %poll_model.title
    )
)]
#[allow(clippy::too_many_lines)]
pub async fn close_and_finalise_poll(
    http: &serenity::Http,
    db: &DatabaseConnection,
    cache: &PollCache,
    mut poll_model: poll::Model,
) -> Result<(), anyhow::Error> {
    let options = poll_option::Entity::find()
        .filter(poll_option::Column::PollId.eq(poll_model.id))
        .all(db)
        .await
        .inspect_err(|e| {
            error!(
                error = ?e,
                "failed to fetch poll options from database during finalisation"
            );
        })
        .context("failed to fetch options for finalising poll")?;

    let votes = vote::Entity::find()
        .filter(vote::Column::PollId.eq(poll_model.id))
        .all(db)
        .await
        .inspect_err(|e| {
            error!(
                error = ?e,
                "failed to fetch poll votes from database during finalisation"
            );
        })
        .context("failed to fetch votes for finalising poll")?;

    let chart = generate_results_chart(&options, &votes);

    let mut description_lines = vec![
        format!("### {}", poll_model.title),
        String::new(), // blank line
        "### **choices**".to_string(),
    ];

    for (index, opt) in options.iter().enumerate() {
        let index_u32 = u32::try_from(index).unwrap_or(0);
        let emoji =
            char::from_u32(0x1F1E6 + index_u32).map_or_else(|| "🔹".to_string(), |c| c.to_string());

        description_lines.push(format!("{emoji} {}", opt.label));
    }

    if let Some(role_id) = poll_model.required_role_id {
        description_lines.push(String::new());
        description_lines.push("### **required role**".to_string());
        description_lines.push(format!("<@&{role_id}>"));
    }

    description_lines.push(String::new());
    description_lines.push("### **result**".to_string());
    description_lines.push(chart);

    let description = description_lines.join("\n");

    let results_embed = serenity::CreateEmbed::new()
        .description(description)
        .color(serenity::Colour::RED);

    // update the original poll message (remove buttons, add final results embed)
    if let Some(msg_id) = poll_model.message_id {
        let channel_id = serenity::ChannelId::new(poll_model.channel_id.cast_unsigned());
        let message_id = serenity::MessageId::new(msg_id.cast_unsigned());

        let builder = serenity::EditMessage::new()
            .embed(results_embed)
            .components(vec![]); // removes buttons

        if let Err(e) = channel_id.edit_message(http, message_id, builder).await {
            error!(
                error = ?e,
                channel_id = channel_id.get(),
                message_id = message_id.get(),
                "failed to update poll discord message with final results"
            );
        }
    }

    // mark active = false in db and update cache AFTER discord ui attempt
    let mut am: poll::ActiveModel = poll_model.clone().into();
    am.is_active = Set(false);

    poll_model = am
        .update(db)
        .await
        .inspect_err(|e| {
            error!(
                error = ?e,
                "failed to update poll status to inactive in database"
            );
        })
        .context("failed to mark poll as inactive in database")?;

    cache.remove(&poll_model.id);

    if let Ok(Some(guild_config)) = guild::Entity::find_by_id(poll_model.guild_id).one(db).await
        && let Some(log_channel_id) = guild_config.log_channel_id
    {
        let log_channel = serenity::ChannelId::new(log_channel_id.cast_unsigned());
        let guild_id = serenity::GuildId::new(poll_model.guild_id.cast_unsigned());

        let mut grouped_votes: HashMap<uuid::Uuid, Vec<i64>> = HashMap::new();
        for opt in &options {
            grouped_votes.insert(opt.id, Vec::new());
        }

        for v in &votes {
            grouped_votes
                .entry(v.option_id)
                .or_default()
                .push(v.user_id);
        }

        let mut log_content = format!(
            "**poll closed: {}**\n*the following votes were cast:*\n```text\n",
            poll_model.title
        );

        for (i, opt) in options.iter().enumerate() {
            let _ = writeln!(log_content, "{}", opt.label);
            let _ = writeln!(log_content, "----");

            if let Some(user_ids) = grouped_votes.get(&opt.id) {
                if user_ids.is_empty() {
                    let _ = writeln!(log_content, "no one voted for this");
                } else {
                    for &id in user_ids {
                        let user_id = serenity::UserId::new(id.cast_unsigned());

                        let display_name = guild_id.member(http, user_id).await.map_or_else(
                            |_| format!("Unknown User ({id})"),
                            |member| member.display_name().to_owned(),
                        );

                        let _ = writeln!(log_content, "{display_name}");
                    }
                }
            }

            if i < options.len() - 1 {
                let _ = writeln!(log_content);
            }
        }

        let _ = write!(log_content, "```");

        // truncate to 2000 chars if vote list grew too long
        if log_content.len() > 1950 {
            log_content.truncate(1900);
            log_content.push_str("\n... [truncated due to discord message limit]\n```");
        }

        if let Err(e) = log_channel
            .send_message(http, serenity::CreateMessage::new().content(log_content))
            .await
        {
            error!(
                error = ?e,
                log_channel_id = log_channel.get(),
                "failed to send poll closing audit log to channel"
            );
        }
    }

    info!("poll successfully finalised and closed");
    Ok(())
}
