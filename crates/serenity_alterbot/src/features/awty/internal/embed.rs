use awty::ModpackCheckReport;
use poise::serenity_prelude::{Colour, CreateEmbed, Timestamp};
use std::fmt::Write;

pub fn create_awty_embed(report: &ModpackCheckReport, add_percentage: bool) -> CreateEmbed {
    let mut mods: Vec<_> = report.mods.iter().collect();
    mods.sort_by(|a, b| {
        let name_a = a.name.as_deref().unwrap_or(&a.id);
        let name_b = b.name.as_deref().unwrap_or(&b.id);
        name_a.cmp(name_b)
    });

    let total = report.total_mods;
    let enabled_count = report.supported_mods_count;

    let mut new_content = String::new();
    let mut hidden_count = 0;
    let max_len = 3900;

    for mod_info in &mods {
        let mod_name = mod_info.name.as_deref().unwrap_or(&mod_info.id);
        let mark = if mod_info.supports_target_version {
            "✅"
        } else {
            "❌"
        };
        let line = format!("{mod_name} - {mark}\n");

        if new_content.len() + line.len() > max_len {
            hidden_count += 1;
        } else {
            new_content.push_str(&line);
        }
    }

    if hidden_count > 0 {
        let _ = write!(new_content, "\n... and {hidden_count} more mods\n");
    }

    if total > 0 {
        if add_percentage {
            let percent = (enabled_count * 100_usize).checked_div(total).unwrap_or(0);
            let _ = write!(
                new_content,
                "\ntotal: {enabled_count}/{total} ({percent}%)\n"
            );
        } else {
            let _ = write!(new_content, "\ntotal: {enabled_count}/{total}\n");
        }
    }

    CreateEmbed::new()
        .title(format!(
            "update status for {}",
            report.target_minecraft_version
        ))
        .description(format!("```text\n{new_content}\n```"))
        .colour(Colour::BLURPLE)
        .timestamp(Timestamp::now())
}
