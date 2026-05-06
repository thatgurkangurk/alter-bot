use poise::serenity_prelude as serenity;
use sea_orm::sea_query::OnConflict;
use sea_orm::{Set, entity::prelude::*};

use crate::bot::{Context, Error};
use crate::models::{guild, voter_ban};

/// configure server-specific bot settings
#[poise::command(
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only,
    subcommands("set_log_channel", "ban_voter", "unban_voter")
)]
#[allow(clippy::unused_async)]
pub async fn settings(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// set the admin channel where finished poll results are logged
#[poise::command(slash_command)]
pub async fn set_log_channel(
    ctx: Context<'_>,
    #[description = "the channel to send poll results to"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("must be run in a guild")?
        .get()
        .cast_signed();
    let channel_id = channel.id.get().cast_signed();

    let active_guild = guild::ActiveModel {
        id: Set(guild_id),
        log_channel_id: Set(Some(channel_id)),
    };

    guild::Entity::insert(active_guild)
        .on_conflict(
            OnConflict::column(guild::Column::Id)
                .update_column(guild::Column::LogChannelId)
                .to_owned(),
        )
        .exec(&ctx.data().db)
        .await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!(
                "poll results will now be logged in <#{channel_id}>."
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// prevent a specific user from voting in this server
#[poise::command(slash_command)]
pub async fn ban_voter(
    ctx: Context<'_>,
    #[description = "user to ban from voting"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("must be run in a guild")?
        .get()
        .cast_signed();
    let user_id = user.id.get().cast_signed();

    ensure_guild_exists(&ctx.data().db, guild_id).await?;

    let ban = voter_ban::ActiveModel {
        guild_id: Set(guild_id),
        user_id: Set(user_id),
    };

    voter_ban::Entity::insert(ban)
        .on_conflict(
            OnConflict::columns([voter_ban::Column::GuildId, voter_ban::Column::UserId])
                .do_nothing()
                .to_owned(),
        )
        .exec(&ctx.data().db)
        .await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!(
                "<@{user_id}> has been banned from voting in polls."
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Allow a previously banned user to vote again
#[poise::command(slash_command)]
pub async fn unban_voter(
    ctx: Context<'_>,
    #[description = "user to unban"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("must be run in a guild")?
        .get()
        .cast_signed();
    let user_id = user.id.get().cast_signed();

    // Delete the composite key entry
    let delete_result = voter_ban::Entity::delete_by_id((guild_id, user_id))
        .exec(&ctx.data().db)
        .await?;

    let response = if delete_result.rows_affected > 0 {
        format!("✅ <@{user_id}> is now allowed to vote again.")
    } else {
        format!("⚠️ <@{user_id}> was not banned.")
    };

    ctx.send(
        poise::CreateReply::default()
            .content(response)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// helper function to ensure a guild row exists before inserting a `voter_ban`
/// this prevents foreign key constraint failures.
async fn ensure_guild_exists(db: &sea_orm::DatabaseConnection, guild_id: i64) -> Result<(), DbErr> {
    let active_guild = guild::ActiveModel {
        id: Set(guild_id),
        log_channel_id: sea_orm::ActiveValue::NotSet,
    };

    guild::Entity::insert(active_guild)
        .on_conflict(
            OnConflict::column(guild::Column::Id)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}
