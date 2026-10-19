use anyhow::anyhow;
use poise::{CreateReply, Modal, serenity_prelude as serenity};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::internal::renderer::generate_results_chart;
use crate::bot::{Context, Data, Error};
use crate::features::polls::internal::logic::{CreatePollParams, PollChoice, create_and_post_poll};
use crate::features::polls::modal::NewPollModal;
use crate::models::{poll, poll_option, vote};

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

            super::internal::logic::close_and_finalise_poll(
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
    let db = &ctx.data().db;

    let poll_opt = poll::Entity::find()
        .filter(poll::Column::MessageId.eq(Some(msg_id)))
        .one(db)
        .await?;

    let Some(active_poll) = poll_opt else {
        ctx.send(poise::CreateReply::default().content("that message is not a poll"))
            .await?;
        return Ok(());
    };

    // 1. Fetch options and votes
    let options = poll_option::Entity::find()
        .filter(poll_option::Column::PollId.eq(active_poll.id))
        .all(db)
        .await?;

    let votes = vote::Entity::find()
        .filter(vote::Column::PollId.eq(active_poll.id))
        .all(db)
        .await?;

    // 2. Generate the dynamic chart
    let chart = generate_results_chart(&options, &votes);

    let mut grouped_votes: std::collections::HashMap<uuid::Uuid, Vec<i64>> =
        std::collections::HashMap::new();
    for opt in &options {
        grouped_votes.insert(opt.id, Vec::new());
    }
    for v in &votes {
        grouped_votes
            .entry(v.option_id)
            .or_default()
            .push(v.user_id);
    }

    let guild_id = ctx.guild_id().ok_or("no guild id")?;
    let http = ctx.http();

    let mut description_lines = vec![
        "### live results".to_string(),
        chart,
        String::new(),
        "### **voter breakdown**".to_string(),
    ];

    for (index, opt) in options.iter().enumerate() {
        let index_u32 = u32::try_from(index).unwrap_or(0);

        let emoji =
            char::from_u32(0x1F1E6 + index_u32).map_or_else(|| "🔹".to_string(), |c| c.to_string());

        description_lines.push(format!("{emoji} **{}**", opt.label));

        let user_ids = grouped_votes
            .get(&opt.id)
            .ok_or_else(|| Error::from(anyhow::anyhow!("missing votes for option {}", opt.id)))?;
        if user_ids.is_empty() {
            description_lines.push("nobody yet".to_string());
        } else {
            for &id in user_ids {
                let uid = serenity::UserId::new(id.cast_unsigned());
                let name = guild_id.member(http, uid).await.map_or_else(
                    |_| format!("unknown user ({id})"),
                    |m| m.display_name().to_owned(),
                );
                description_lines.push(format!("- {name}"));
            }
        }
        description_lines.push(String::new());
    }

    if let Some(role_id) = active_poll.required_role_id {
        description_lines.push("### **required role**".to_string());
        description_lines.push(format!("<@&{role_id}>"));
    }

    let status_embed = serenity::CreateEmbed::new()
        .title(format!("live status: {}", active_poll.title))
        .description(description_lines.join("\n"))
        .color(serenity::Colour::BLUE);

    ctx.send(poise::CreateReply::default().embed(status_embed))
        .await?;

    Ok(())
}

/// start a new poll
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", guild_only)]
async fn start_poll(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "channel to post the poll in"] target_channel: serenity::GuildChannel,
    #[description = "how long the poll should run (in minutes)"] duration_minutes: i64,
    #[description = "a role that members must have to vote (optional)"] required_role_id: Option<
        serenity::RoleId,
    >,
) -> Result<(), Error> {
    let data = NewPollModal::execute(ctx).await?;
    let Some(data) = data else { return Ok(()) };

    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow!("must be run in a guild"))?;

    let mut raw_inputs = vec![Some(data.opt_1), Some(data.opt_2), data.opt_3];
    if let Some(bulk) = data.opt_bulk {
        raw_inputs.extend(bulk.split(',').map(|s| Some(s.trim().to_string())));
    }

    let choices: Vec<PollChoice> = raw_inputs
        .into_iter()
        .filter_map(|opt| opt?.parse().ok())
        .collect();

    let poll = create_and_post_poll(
        &ctx.data().db,
        ctx.http(),
        CreatePollParams {
            title: data.title,
            guild_id,
            target_channel_id: target_channel.id,
            duration_minutes,
            required_role_id,
            choices,
        },
    )
    .await?;

    let ends_at = poll.ends_at.to_utc();

    ctx.data().cache.insert(poll.id, ends_at);

    let reply = CreateReply::default()
        .content("poll created !")
        .reply(true)
        .ephemeral(true);

    ctx.send(reply).await?;
    Ok(())
}

pub fn poll_commands(mut cmds: Vec<crate::bot::Command>) -> Vec<crate::bot::Command> {
    cmds.push(start_poll());
    cmds.push(end_poll_command());
    cmds.push(check_poll_status());

    cmds
}
