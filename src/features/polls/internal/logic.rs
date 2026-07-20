use super::renderer::generate_results_chart;
use crate::bot::Error;
use crate::models::{guild, poll, poll_option, vote};
use poise::serenity_prelude as serenity;
use sea_orm::{ActiveModelTrait, QueryFilter, Set, entity::prelude::*};
use std::collections::HashMap;
use std::fmt::Write;

pub async fn close_and_finalize_poll(
    http: &serenity::Http,
    db: &DatabaseConnection,
    cache: &crate::bot::PollCache,
    mut poll_model: poll::Model,
) -> Result<(), Error> {
    let mut am: poll::ActiveModel = poll_model.clone().into();
    am.is_active = Set(false);
    poll_model = am.update(db).await?;

    cache.write().await.remove(&poll_model.id);

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
