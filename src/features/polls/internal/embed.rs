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

    for (index, label) in options.iter().enumerate() {
        let index_u32 = u32::try_from(index).unwrap_or(0);

        let emoji =
            char::from_u32(0x1F1E6 + index_u32).map_or_else(|| "🔹".to_string(), |c| c.to_string());

        description_lines.push(format!("{emoji} **{label}**"));
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
