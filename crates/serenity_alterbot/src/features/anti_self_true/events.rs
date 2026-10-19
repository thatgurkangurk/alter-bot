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
    let serenity::FullEvent::ReactionAdd { add_reaction } = event else {
        return Ok(());
    };

    let (Some(user_id), Some(author_id)) = (add_reaction.user_id, add_reaction.message_author_id)
    else {
        return Ok(());
    };

    if user_id != author_id || !is_blocked_emoji(&add_reaction.emoji) {
        return Ok(());
    }

    let config = data.config_manager.get().await;
    let Some(misc) = &config.misc else {
        add_reaction.delete(&ctx.http).await?;
        return Ok(());
    };

    let allowed_roles = misc.self_reaction_roles.as_deref().unwrap_or(&[]);
    let is_blacklist_mode = misc.invert_self_reaction_roles.unwrap_or(false);

    let has_matching_role = add_reaction
        .member
        .as_ref()
        .is_some_and(|member| member.roles.iter().any(|r| allowed_roles.contains(r)));

    let is_allowed = has_matching_role ^ is_blacklist_mode;

    if !is_allowed {
        add_reaction.delete(&ctx.http).await?;
    }

    Ok(())
}
