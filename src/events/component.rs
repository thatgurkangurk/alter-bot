use chrono::Utc;
use poise::serenity_prelude as serenity;
use sea_orm::sea_query::OnConflict;
use sea_orm::{Set, entity::prelude::*};
use uuid::Uuid;

use crate::bot::{Data, Error};
use crate::models::{poll, vote, voter_ban};
use crate::utils::embeds::build_poll_embed;

pub async fn handle(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    // we only care about InteractionCreate events
    if let serenity::FullEvent::InteractionCreate { interaction } = event {
        // specifically, we want button interactions
        if let Some(component) = interaction.as_message_component() {
            let custom_id = &component.data.custom_id;

            // check if this is one of our vote buttons
            if custom_id.starts_with("vote_") {
                let parts: Vec<&str> = custom_id.split('_').collect();
                if parts.len() == 3 {
                    let choice_str = parts[1];
                    let poll_id_str = parts[2];

                    let Ok(poll_id) = Uuid::parse_str(poll_id_str) else {
                        return Ok(());
                    };

                    let user_id = component.user.id.get().cast_signed();
                    let guild_id = component.guild_id.map_or(0, |id| id.get().cast_signed());

                    let is_banned = voter_ban::Entity::find_by_id((guild_id, user_id))
                        .one(&data.db)
                        .await?
                        .is_some();

                    if is_banned {
                        return reply_ephemeral(
                            ctx,
                            component,
                            "you are banned from voting in this server.",
                        )
                        .await;
                    }

                    // check if the poll is still active (prevents race conditions)
                    let active_poll = poll::Entity::find_by_id(poll_id).one(&data.db).await?;

                    let p = if let Some(p) = active_poll {
                        if !p.is_active {
                            return reply_ephemeral(ctx, component, "this poll has already ended")
                                .await;
                        }
                        p
                    } else {
                        return reply_ephemeral(ctx, component, "that poll wasn't found").await;
                    };

                    let choice = match choice_str {
                        "Yes" => vote::VoteChoice::Yes,
                        "No" => vote::VoteChoice::No,
                        "HardNo" => vote::VoteChoice::HardNo,
                        _ => return Ok(()),
                    };

                    let new_vote = vote::ActiveModel {
                        poll_id: Set(poll_id),
                        user_id: Set(user_id),
                        choice: Set(choice.clone()),
                    };

                    vote::Entity::insert(new_vote)
                        .on_conflict(
                            OnConflict::columns([vote::Column::PollId, vote::Column::UserId])
                                .update_column(vote::Column::Choice)
                                .to_owned(),
                        )
                        .exec(&data.db)
                        .await?;

                    let total_votes = vote::Entity::find()
                        .filter(vote::Column::PollId.eq(poll_id))
                        .count(&data.db)
                        .await?;

                    let ends_at_utc = p.ends_at.with_timezone(&Utc);

                    let updated_embed = build_poll_embed(&p.title, ends_at_utc, total_votes);

                    let response =
                        serenity::CreateInteractionResponseMessage::new().embed(updated_embed);

                    component
                        .create_response(
                            &ctx.http,
                            serenity::CreateInteractionResponse::UpdateMessage(response),
                        )
                        .await?;

                    component
                        .create_followup(
                            &ctx.http,
                            serenity::CreateInteractionResponseFollowup::new()
                                .content(format!(
                                    "your vote for **{choice_str}** has been recorded !"
                                ))
                                .ephemeral(true),
                        )
                        .await?;
                }
            }
        }
    }
    Ok(())
}
/// helper function to send an ephemeral reply to a component interaction
async fn reply_ephemeral(
    ctx: &serenity::Context,
    component: &serenity::ComponentInteraction,
    message: &str,
) -> Result<(), Error> {
    let response = serenity::CreateInteractionResponseMessage::new()
        .content(message)
        .ephemeral(true);

    component
        .create_response(
            &ctx.http,
            serenity::CreateInteractionResponse::Message(response),
        )
        .await?;
    Ok(())
}
