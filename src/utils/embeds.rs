use ::serenity::model::Colour;
use chrono::{DateTime, Utc};
use poise::serenity_prelude as serenity;

pub fn build_poll_embed(
    title: &str,
    ends_at: DateTime<Utc>,
    total_votes: u64,
) -> serenity::CreateEmbed {
    let discord_timestamp = format!("<t:{}:R>", ends_at.timestamp());

    serenity::CreateEmbed::new()
        .title(title)
        .description(format!(
            "please cast your vote below.\nthis poll ends {discord_timestamp}."
        ))
        .color(Colour::ORANGE)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "current voters: {total_votes}"
        )))
}
