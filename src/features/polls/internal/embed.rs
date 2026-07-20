use crate::emojis::{HARD_NO, NO, YES};
use chrono::{DateTime, Utc};
use poise::serenity_prelude as serenity;

pub fn build_poll_embed(
    title: &str,
    ends_at: DateTime<Utc>,
    total_votes: u64,
    options: &[String],
    required_role: Option<serenity::RoleId>,
) -> serenity::CreateEmbed {
    let timestamp = format!("<t:{}:R>", ends_at.timestamp());

    let mut description_lines = vec![
        format!("### {title}"),
        String::new(),
        "### **choices**".to_string(),
    ];

    for label in options {
        let prefix = match label.to_lowercase().as_str() {
            "yes" => YES.text,
            "no" => NO.text,
            "hardno" | "hard no" => HARD_NO.text,
            _ => "🔹", // fallback emoji for other options
        };
        description_lines.push(format!("{prefix} {label}"));
    }

    if let Some(role_id) = required_role {
        description_lines.push(String::new());
        description_lines.push("### **required role**".to_string());
        description_lines.push(format!("<@&{role_id}>"));
    }

    description_lines.push(String::new());
    description_lines.push(format!(
        "please cast your vote below.\nthis poll ends {timestamp}."
    ));

    let description = description_lines.join("\n");

    serenity::CreateEmbed::new()
        .description(description)
        .color(serenity::Colour::ORANGE)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "current voters: {total_votes}"
        )))
}
