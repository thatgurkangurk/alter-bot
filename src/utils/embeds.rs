use ::serenity::model::Colour;
use chrono::{DateTime, Utc};
use poise::serenity_prelude as serenity;

use crate::emojis::{HARD_NO, NO, YES};

pub fn build_poll_embed(
    title: &str,
    ends_at: DateTime<Utc>,
    total_votes: u64,
) -> serenity::CreateEmbed {
    let timestamp = format!("<t:{}:R>", ends_at.timestamp());

    let description = [
        format!("### {title}"),
        String::new(), // blank line
        "### **choices**".to_string(),
        format!("{} yes", YES.text),
        format!("{} no", NO.text),
        format!("{} hard no", HARD_NO.text),
        String::new(),
        format!("please cast your vote below.\nthis poll ends {timestamp}."),
    ]
    .join("\n");

    serenity::CreateEmbed::new()
        .description(description)
        .color(Colour::ORANGE)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "current voters: {total_votes}"
        )))
}
