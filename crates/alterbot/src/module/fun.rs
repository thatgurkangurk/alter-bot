use crate::module::{Module, ModuleEntry};
use anyhow::Context;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::{
    channel::message::EmojiReactionType,
    gateway::event::Event,
    id::{Id, marker::EmojiMarker},
};

pub struct FunModule;

pub const NAME: &str = "fun";

const BLOCKED_CUSTOM_EMOJI_IDS: &[Id<EmojiMarker>] =
    &[Id::new(1494754728582709338), Id::new(1500871782477856969)];
const BLOCKED_UNICODE_EMOJIS: &[&str] = &["✅", "❌"];

fn is_blocked_emoji(emoji: &EmojiReactionType) -> bool {
    match emoji {
        EmojiReactionType::Unicode { name } => BLOCKED_UNICODE_EMOJIS.contains(&name.as_str()),
        EmojiReactionType::Custom { id, .. } => BLOCKED_CUSTOM_EMOJI_IDS.contains(id),
    }
}

const IM_TRIGGERS: &[&str] = &["im", "i'm", "i’m", "i am"];
const DELIMITERS: &[char] = &['.', ',', '\n', '!', '?'];
const MAX_LENGTH: usize = 100;

fn get_dad_joke_name(content: &str) -> Option<String> {
    let content = content.trim();
    let lower = content.to_lowercase();
    let mut start_index: Option<usize> = None;

    // collect char indices to safely check lookbehind for word boundaries
    let chars: Vec<(usize, char)> = lower.char_indices().collect();

    for (i, &(byte_idx, _)) in chars.iter().enumerate() {
        let is_boundary = i == 0 || chars[i - 1].1.is_whitespace();

        if is_boundary {
            for trigger in IM_TRIGGERS {
                if lower[byte_idx..].starts_with(trigger) {
                    let after_byte_idx = byte_idx + trigger.len(); // trigger.len() is byte length

                    let is_end_boundary = after_byte_idx == lower.len()
                        || lower[after_byte_idx..].starts_with(|c: char| c.is_whitespace());

                    if is_end_boundary {
                        start_index = Some(after_byte_idx);
                    }
                }
            }
        }
    }

    let mut start_byte = start_index?;

    if let Some(offset) = content[start_byte..].find(|c: char| !c.is_whitespace()) {
        start_byte += offset;
    } else {
        start_byte = content.len();
    }

    if start_byte >= content.len() {
        return None;
    }

    let remainder = &content[start_byte..];

    let end = remainder.find(DELIMITERS).unwrap_or(remainder.len());
    let result = remainder[..end].trim();

    if result.is_empty() {
        return None;
    }

    let truncated: String = result.chars().take(MAX_LENGTH).collect();
    let final_name = truncated.trim().to_string();

    if final_name.is_empty() {
        None
    } else {
        Some(final_name)
    }
}

#[async_trait::async_trait]
impl Module for FunModule {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn handle_event(&self, event: &Event) -> anyhow::Result<()> {
        match event {
            Event::MessageCreate(msg) => {
                if msg.author.bot {
                    return Ok(());
                }

                let Some(name) = get_dad_joke_name(&msg.content) else {
                    return Ok(());
                };

                let current_user = crate::CTX
                    .cache
                    .current_user()
                    .context("current user missing from cache")?;

                let bot_name = current_user
                    .global_name
                    .as_deref()
                    .unwrap_or(&current_user.name);

                let name = crate::util::sanitise_pings(&name);

                let content = format!("Hi {}, I'm {}!", name, bot_name);

                crate::CTX
                    .http
                    .create_message(msg.channel_id)
                    .content(&content)
                    .reply(msg.id)
                    .await?;
            }

            Event::ReactionAdd(reaction) => {
                let Some(author_id) = reaction.message_author_id else {
                    return Ok(());
                };

                if reaction.user_id != author_id || !is_blocked_emoji(&reaction.emoji) {
                    return Ok(());
                }

                let request_emoji = match &reaction.emoji {
                    EmojiReactionType::Custom { id, name, .. } => RequestReactionType::Custom {
                        id: *id,
                        name: name.as_deref(),
                    },
                    EmojiReactionType::Unicode { name } => RequestReactionType::Unicode { name },
                };

                let delete_reaction = async {
                    crate::CTX
                        .http
                        .delete_reaction(
                            reaction.channel_id,
                            reaction.message_id,
                            &request_emoji,
                            reaction.user_id,
                        )
                        .await
                };

                let config = crate::CTX.config_manager.get_sync();
                let Some(misc) = &config.misc else {
                    delete_reaction.await?;
                    return Ok(());
                };

                let allowed_roles = misc.self_reaction_roles.as_deref().unwrap_or(&[]);
                let is_blacklist_mode = misc.invert_self_reaction_roles.unwrap_or(false);

                let has_matching_role = reaction.member.as_ref().is_some_and(|member| {
                    member
                        .roles
                        .iter()
                        .any(|r| allowed_roles.contains(&r.get()))
                });

                let is_allowed = has_matching_role ^ is_blacklist_mode;

                if !is_allowed {
                    delete_reaction.await?;
                }
            }

            _ => {}
        }

        Ok(())
    }
}

inventory::submit! {
    ModuleEntry {
        module: &FunModule
    }
}
