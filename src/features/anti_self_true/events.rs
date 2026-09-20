#![allow(clippy::unreadable_literal)]

use crate::bot::{Data, Error};
use poise::serenity_prelude as serenity;

const BLOCKED_CUSTOM_EMOJI_IDS: &[u64] = &[1494754728582709338, 1500871782477856969];

const BLOCKED_UNICODE_EMOJIS: &[&str] = &["✅", "❌"];

fn is_blocked_emoji(emoji: &serenity::ReactionType) -> bool {
    match emoji {
        serenity::ReactionType::Unicode(str) => BLOCKED_UNICODE_EMOJIS.contains(&str.as_str()),
        serenity::ReactionType::Custom { id, .. } => BLOCKED_CUSTOM_EMOJI_IDS.contains(&id.get()),
        _ => false,
    }
}

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let serenity::FullEvent::ReactionAdd { add_reaction } = event {
        let (Some(user_id), Some(author_id)) =
            (add_reaction.user_id, add_reaction.message_author_id)
        else {
            return Ok(());
        };

        if user_id == author_id && is_blocked_emoji(&add_reaction.emoji) {
            let config = data.config_manager.get().await;
            let message = add_reaction.message(&ctx.http).await?;

            if message.author.id == author_id {
                let role_id = config
                    .misc
                    .as_ref()
                    .and_then(|m| m.can_react_true_to_own_messages_role);

                let has_role = match (role_id, &add_reaction.member) {
                    (Some(required_role), Some(member)) => member.roles.contains(&required_role),
                    _ => false,
                };

                if !has_role {
                    add_reaction.delete(&ctx.http).await?;
                }
            }
        }
    }

    Ok(())
}
