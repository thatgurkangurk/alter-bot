use crate::module::{Module, ModuleEntry};
use anyhow::Context;
use twilight_model::gateway::event::Event;

pub struct FunModule;

pub const NAME: &str = "fun";

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
        if let Event::MessageCreate(msg) = event {
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

        Ok(())
    }
}

inventory::submit! {
    ModuleEntry {
        module: &FunModule
    }
}
