use chrono::{Duration, Utc};
use poise::serenity_prelude as serenity;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::fmt::Write;
use uuid::Uuid;

use super::internal::embed::build_poll_embed;
use super::internal::renderer::generate_results_chart;
use crate::bot::{Context, Data, Error};
use crate::emojis::{HARD_NO, NO, YES};
use crate::models::{poll, vote};

#[poise::command(
    context_menu_command = "End Poll",
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
async fn end_poll_command(
    ctx: crate::bot::Context<'_>,
    #[description = "The poll message to end"] message: serenity::Message,
) -> Result<(), Error> {
    let msg_id = message.id.get().cast_signed();

    let poll_opt = poll::Entity::find()
        .filter(poll::Column::MessageId.eq(Some(msg_id)))
        .filter(poll::Column::IsActive.eq(true))
        .one(&ctx.data().db)
        .await?;

    match poll_opt {
        Some(poll_model) => {
            ctx.send(
                poise::CreateReply::default()
                    .content("closing the poll...")
                    .ephemeral(true),
            )
            .await?;

            super::internal::logic::close_and_finalize_poll(
                ctx.http(),
                &ctx.data().db,
                &ctx.data().cache,
                poll_model,
            )
            .await?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default()
                    .content("❌ that message isn't an active poll.")
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

/// check the current results and voters of a poll without closing it
#[poise::command(
    context_menu_command = "Check Poll Status",
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
async fn check_poll_status(
    ctx: Context<'_>,
    #[description = "The poll message to check"] message: serenity::Message,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let msg_id = message.id.get().cast_signed();

    let poll_opt = poll::Entity::find()
        .filter(poll::Column::MessageId.eq(Some(msg_id)))
        .one(&ctx.data().db)
        .await?;

    let Some(active_poll) = poll_opt else {
        ctx.send(poise::CreateReply::default().content("that message is not a poll"))
            .await?;
        return Ok(());
    };

    let votes = vote::Entity::find()
        .filter(vote::Column::PollId.eq(active_poll.id))
        .all(&ctx.data().db)
        .await?;

    let vote_data: Vec<(i64, vote::VoteChoice)> =
        votes.into_iter().map(|v| (v.user_id, v.choice)).collect();

    let chart = generate_results_chart(&vote_data, active_poll.has_hard_no);

    let mut yes_votes = Vec::new();
    let mut no_votes = Vec::new();
    let mut hard_no_votes = Vec::new();

    for (user_id, choice) in &vote_data {
        match choice {
            vote::VoteChoice::Yes => yes_votes.push(*user_id),
            vote::VoteChoice::No => no_votes.push(*user_id),
            vote::VoteChoice::HardNo => hard_no_votes.push(*user_id),
        }
    }

    let guild_id = ctx.guild_id().ok_or("no guild id")?;
    let http = ctx.http();

    let format_users = |users: Vec<i64>| async move {
        if users.is_empty() {
            return "nobody yet\n".to_string();
        }

        let mut list = String::new();
        for id in users {
            let uid = serenity::UserId::new(id.cast_unsigned());
            let name = guild_id.member(http, uid).await.map_or_else(
                |_| format!("unknown user ({id})"),
                |m| m.display_name().to_owned(),
            );
            let _ = writeln!(list, "- {name}");
        }
        list
    };

    let yes_list = format_users(yes_votes).await;
    let no_list = format_users(no_votes).await;
    let hard_no_list = format_users(hard_no_votes).await;

    let mut description_lines = vec![
        "### live results".to_string(),
        chart,
        String::new(), // blank line
        "### **voter breakdown**".to_string(),
        format!("{}", crate::emojis::YES.text),
        yes_list,
        format!("{}", crate::emojis::NO.text),
        no_list,
    ];

    if active_poll.has_hard_no {
        description_lines.push(crate::emojis::HARD_NO.text.to_string());
        description_lines.push(hard_no_list);
    }

    if let Some(role_id) = active_poll.required_role_id {
        description_lines.push(String::new());
        description_lines.push("### **required role**".to_string());
        description_lines.push(format!("<@&{role_id}>"));
    }

    let description = description_lines.join("\n");

    let status_embed = serenity::CreateEmbed::new()
        .title(format!("live status: {}", active_poll.title))
        .description(description)
        .color(serenity::Colour::BLUE);

    ctx.send(poise::CreateReply::default().embed(status_embed))
        .await?;

    Ok(())
}

/// start a new poll
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", guild_only)]
async fn start_poll(
    ctx: Context<'_>,
    #[description = "poll title"] poll_title: String,
    #[description = "channel to post the poll in"] target_channel: serenity::GuildChannel,
    #[description = "how long the poll should run (in minutes)"] duration_minutes: i64,
    #[description = "include the 'hard no' option? (defaults to false)"] include_hard_no: Option<
        bool,
    >,
    #[description = "a role that members must have to vote (optional)"] required_role_id: Option<
        serenity::RoleId,
    >,
) -> Result<(), Error> {
    let include_hard_no = include_hard_no.unwrap_or(false);
    let ends_at = Utc::now() + Duration::minutes(duration_minutes);
    let guild_id = ctx.guild_id().ok_or("must be run in a guild")?;

    let poll_id = Uuid::new_v4();

    let mut buttons = vec![
        serenity::CreateButton::new(format!("vote_Yes_{poll_id}"))
            .emoji(YES.id)
            .style(serenity::ButtonStyle::Secondary),
        serenity::CreateButton::new(format!("vote_No_{poll_id}"))
            .emoji(NO.id)
            .style(serenity::ButtonStyle::Secondary),
    ];

    if include_hard_no {
        buttons.push(
            serenity::CreateButton::new(format!("vote_HardNo_{poll_id}"))
                .emoji(HARD_NO.id)
                .style(serenity::ButtonStyle::Secondary),
        );
    }

    let components = vec![serenity::CreateActionRow::Buttons(buttons)];

    let embed = build_poll_embed(&poll_title, ends_at, 0, include_hard_no, required_role_id);

    let poll_message = serenity::CreateMessage::new()
        .embed(embed)
        .components(components);

    let msg = target_channel
        .id
        .send_message(ctx.http(), poll_message)
        .await?;

    let new_poll = poll::ActiveModel {
        id: Set(poll_id),
        guild_id: Set(guild_id.get().cast_signed()),
        channel_id: Set(target_channel.id.get().cast_signed()),
        message_id: Set(Some(msg.id.get().cast_signed())),
        title: Set(poll_title),
        ends_at: Set(ends_at.into()),
        is_active: Set(true),
        has_hard_no: Set(include_hard_no),
        required_role_id: Set(required_role_id.map(|r| r.get().cast_signed())),
    };

    new_poll.insert(&ctx.data().db).await?;

    ctx.data().cache.write().await.insert(poll_id, ends_at);

    ctx.send(
        poise::CreateReply::default()
            .content(format!(
                "successfully created poll in <#{}>",
                target_channel.id.get()
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

pub fn poll_commands() -> Vec<poise::Command<Data, Error>> {
    vec![start_poll(), end_poll_command(), check_poll_status()]
}
