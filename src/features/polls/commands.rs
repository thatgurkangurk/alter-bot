use chrono::{Duration, Utc};
use poise::{Modal, serenity_prelude as serenity};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use super::internal::embed::build_poll_embed;
use super::internal::renderer::generate_results_chart;
use crate::bot::{Context, Data, Error};
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

    for opt in &options {
        let prefix = match opt.label.to_lowercase().as_str() {
            "yes" => crate::emojis::YES.text.to_string(),
            "no" => crate::emojis::NO.text.to_string(),
            "hardno" | "hard no" => crate::emojis::HARD_NO.text.to_string(),
            _ => "🔹".to_string(),
        };

        description_lines.push(format!("{prefix} **{}**", opt.label));

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
    use super::modal::parse_weight;

    let data = NewPollModal::execute(ctx).await?;
    let Some(data) = data else { return Ok(()) };

    let ends_at = Utc::now() + Duration::minutes(duration_minutes);
    let poll_id = Uuid::new_v4();
    let guild_id = ctx.guild_id().ok_or("must be run in a guild")?;

    let mut raw_inputs = vec![Some(data.opt_1), Some(data.opt_2), data.opt_3];
    if let Some(bulk) = data.opt_bulk {
        raw_inputs.extend(bulk.split(',').map(|s| Some(s.trim().to_string())));
    }

    let mut poll_opts = Vec::new();
    let mut buttons = Vec::new();
    let mut labels = Vec::new();

    for raw in raw_inputs.into_iter().flatten() {
        if raw.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = raw.split(':').collect();
        let label = parts[0].trim().to_string();
        let weight_opt = parts.get(1).map(ToString::to_string);
        let weight = parse_weight(weight_opt);

        let opt_id = Uuid::new_v4();
        poll_opts.push(poll_option::ActiveModel {
            id: Set(opt_id),
            poll_id: Set(poll_id),
            label: Set(label.clone()),
            weight: Set(weight),
        });

        labels.push(label.clone());
        buttons.push(
            serenity::CreateButton::new(format!("vote_{opt_id}_{poll_id}"))
                .label(label)
                .style(serenity::ButtonStyle::Secondary),
        );
    }

    let new_poll = poll::ActiveModel {
        id: Set(poll_id),
        guild_id: Set(guild_id.get().cast_signed()),
        channel_id: Set(target_channel.id.get().cast_signed()),
        title: Set(data.title),
        ends_at: Set(ends_at.into()),
        is_active: Set(true),
        required_role_id: Set(required_role_id.map(|r| r.get().cast_signed())),
        ..Default::default()
    };
    let poll_model = new_poll.insert(&ctx.data().db).await?;

    poll_option::Entity::insert_many(poll_opts)
        .exec(&ctx.data().db)
        .await?;

    let action_rows: Vec<serenity::CreateActionRow> = buttons
        .chunks(5)
        .map(|chunk| serenity::CreateActionRow::Buttons(chunk.to_vec()))
        .collect();

    let embed = build_poll_embed(&poll_model.title, ends_at, 0, &labels, required_role_id);
    let msg = target_channel
        .id
        .send_message(
            ctx.http(),
            serenity::CreateMessage::new()
                .embed(embed)
                .components(action_rows),
        )
        .await?;

    let mut poll_am: poll::ActiveModel = poll_model.into();
    poll_am.message_id = Set(Some(msg.id.get().cast_signed()));
    poll_am.update(&ctx.data().db).await?;
    ctx.data().cache.write().await.insert(poll_id, ends_at);

    ctx.send(
        poise::CreateReply::default()
            .content("poll created!")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

pub fn poll_commands(mut cmds: Vec<crate::bot::Command>) -> Vec<crate::bot::Command> {
    cmds.push(start_poll());
    cmds.push(end_poll_command());
    cmds.push(check_poll_status());

    cmds
}
