use chrono::{Duration, Utc};
use poise::serenity_prelude as serenity;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::bot::{Context, Error};
use crate::emojis::{HARD_NO, NO, YES};
use crate::models::poll;
use crate::utils::embeds::build_poll_embed;

#[poise::command(
    context_menu_command = "End Poll",
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn end_poll_command(
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

            crate::utils::poll_logic::close_and_finalize_poll(
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
/// start a new member poll
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", guild_only)]
pub async fn start_member_poll(
    ctx: Context<'_>,
    #[description = "poll title"] poll_title: String,
    #[description = "channel to post the poll in"] target_channel: serenity::GuildChannel,
    #[description = "how long the poll should run (in minutes)"] duration_minutes: i64,
) -> Result<(), Error> {
    let ends_at = Utc::now() + Duration::minutes(duration_minutes);
    let guild_id = ctx.guild_id().ok_or("must be run in a guild")?;

    let poll_id = Uuid::new_v4();

    let components = vec![serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new(format!("vote_Yes_{poll_id}"))
            .emoji(YES.id)
            .style(serenity::ButtonStyle::Secondary),
        serenity::CreateButton::new(format!("vote_No_{poll_id}"))
            .emoji(NO.id)
            .style(serenity::ButtonStyle::Secondary),
        serenity::CreateButton::new(format!("vote_HardNo_{poll_id}"))
            .emoji(HARD_NO.id)
            .style(serenity::ButtonStyle::Secondary),
    ])];

    let embed = build_poll_embed(&poll_title, ends_at, 0);

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
