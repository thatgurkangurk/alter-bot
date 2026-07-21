use std::collections::HashMap;
use std::fmt::Write;

use anyhow::{Context, Result, anyhow};
use chrono::{Duration, Utc};
use poise::serenity_prelude as serenity;
use sea_orm::{ActiveModelTrait, QueryFilter, Set, entity::prelude::*};

use super::renderer::generate_results_chart;
use crate::bot::Error;
use crate::features::polls::cache::PollCache;
use crate::features::polls::internal::embed::build_poll_embed;
use crate::models::{guild, poll, poll_option, vote};

use tracing::{error, info, instrument};

pub struct CreatePollParams {
    pub title: String,
    pub guild_id: serenity::GuildId,
    pub target_channel_id: serenity::ChannelId,
    pub duration_minutes: i64,
    pub required_role_id: Option<serenity::RoleId>,
    pub raw_inputs: Vec<Option<String>>,
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
    use super::super::modal::parse_weight;

    let ends_at = Utc::now() + Duration::minutes(params.duration_minutes);
    let poll_id = Uuid::new_v4();

    tracing::Span::current().record("poll_id", poll_id.to_string());

    let mut poll_opts = Vec::new();
    let mut buttons = Vec::new();
    let mut labels = Vec::new();

    for (index, raw) in params.raw_inputs.into_iter().flatten().enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split(':').collect();
        let label = parts[0].trim().to_string();
        let weight_opt = parts.get(1).map(|s| s.trim().to_string());
        let weight = parse_weight(weight_opt);

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

pub async fn close_and_finalize_poll(
    http: &serenity::Http,
    db: &DatabaseConnection,
    cache: &PollCache,
    mut poll_model: poll::Model,
) -> Result<(), Error> {
    let mut am: poll::ActiveModel = poll_model.clone().into();
    am.is_active = Set(false);
    poll_model = am.update(db).await?;

    cache.remove(&poll_model.id);

    let options = poll_option::Entity::find()
        .filter(poll_option::Column::PollId.eq(poll_model.id))
        .all(db)
        .await?;

    let votes = vote::Entity::find()
        .filter(vote::Column::PollId.eq(poll_model.id))
        .all(db)
        .await?;

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

    if let Some(msg_id) = poll_model.message_id {
        let channel_id = serenity::ChannelId::new(poll_model.channel_id.cast_unsigned());
        let message_id = serenity::MessageId::new(msg_id.cast_unsigned());

        let builder = serenity::EditMessage::new()
            .embed(results_embed)
            .components(vec![]); // removes the buttons since the poll is closed

        let _ = channel_id.edit_message(&http, message_id, builder).await;
    }

    if let Ok(Some(guild_config)) = guild::Entity::find_by_id(poll_model.guild_id).one(db).await
        && let Some(log_channel_id) = guild_config.log_channel_id
    {
        let log_channel = serenity::ChannelId::new(log_channel_id.cast_unsigned());
        let guild_id = serenity::GuildId::new(poll_model.guild_id.cast_unsigned());

        // group votes by option_id
        let mut grouped_votes: HashMap<uuid::Uuid, Vec<i64>> = HashMap::new();
        for opt in &options {
            grouped_votes.insert(opt.id, Vec::new()); // ensure every option exists in the map
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

        // iterate over the dynamic options to build the log output
        for (i, opt) in options.iter().enumerate() {
            let _ = writeln!(log_content, "{}", opt.label);
            let _ = writeln!(log_content, "----");

            let user_ids = grouped_votes
                .get(&opt.id)
                .ok_or_else(|| anyhow::anyhow!("failed to find votes for option: {}", opt.id))?;

            if user_ids.is_empty() {
                let _ = writeln!(log_content, "no one voted for this");
            } else {
                for &id in user_ids {
                    let user_id = serenity::UserId::new(id.cast_unsigned());

                    // fetch the member object from Discord to get their server nickname
                    let display_name = guild_id.member(&http, user_id).await.map_or_else(
                        |_| format!("Unknown User ({id})"),
                        |member| member.display_name().to_owned(),
                    );

                    let _ = writeln!(log_content, "{display_name}");
                }
            }

            if i < options.len() - 1 {
                let _ = writeln!(log_content);
            }
        }

        let _ = write!(log_content, "```");

        let _ = log_channel
            .send_message(&http, serenity::CreateMessage::new().content(log_content))
            .await;
    }

    Ok(())
}
