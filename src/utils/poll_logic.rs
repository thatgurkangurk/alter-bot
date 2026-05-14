use crate::bot::Error;
use crate::emojis::{HARD_NO, NO, YES};
use crate::models::{guild, poll, vote};
use crate::utils::renderer::generate_results_chart;
use poise::serenity_prelude as serenity;
use sea_orm::{ActiveModelTrait, QueryFilter, Set, entity::prelude::*};
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

    let votes = vote::Entity::find()
        .filter(vote::Column::PollId.eq(poll_model.id))
        .all(db)
        .await?;

    let vote_data: Vec<(i64, vote::VoteChoice)> =
        votes.into_iter().map(|v| (v.user_id, v.choice)).collect();

    let chart = generate_results_chart(&vote_data, poll_model.has_hard_no);

    let description = format!(
        "### {}\n\n### **choices**\n{} yes\n{} no\n{} hard no\n\n### **result**\n{}",
        poll_model.title, YES.text, NO.text, HARD_NO.text, chart
    );

    let results_embed = serenity::CreateEmbed::new()
        .description(description)
        .color(serenity::Colour::RED);

    if let Some(msg_id) = poll_model.message_id {
        let channel_id = serenity::ChannelId::new(poll_model.channel_id.cast_unsigned());
        let message_id = serenity::MessageId::new(msg_id.cast_unsigned());

        let builder = serenity::EditMessage::new()
            .embed(results_embed)
            .components(vec![]);

        let _ = channel_id.edit_message(&http, message_id, builder).await;
    }

    // admin logging
    if let Ok(Some(guild_config)) = guild::Entity::find_by_id(poll_model.guild_id).one(db).await
        && let Some(log_channel_id) = guild_config.log_channel_id
    {
        let log_channel = serenity::ChannelId::new(log_channel_id.cast_unsigned());
        let guild_id = serenity::GuildId::new(poll_model.guild_id.cast_unsigned());

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
            poll_model.title
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

    Ok(())
}
