use std::collections::HashMap;

pub fn format_mod_statuses(results: &HashMap<String, bool>, percentage: bool) -> String {
    let mut mods: Vec<_> = results.keys().collect();
    mods.sort();

    // Calculate totals first
    let total = mods.len();
    let mut enabled_count = 0;
    for status in results.values() {
        if *status {
            enabled_count += 1;
        }
    }

    let mut new_content = String::new();
    let mut hidden_count = 0;

    // Embeds allow 4096 chars. We stop at 3900 to leave plenty of room
    // for the total count string and markdown formatting.
    let max_len = 3900;

    for mod_name in &mods {
        let status = results.get(*mod_name).unwrap_or(&false);
        let mark = if *status { "✅" } else { "❌" };
        let line = format!("{} - {}\n", mod_name, mark);

        // If adding this line pushes us over the limit, count it as hidden instead
        if new_content.len() + line.len() > max_len {
            hidden_count += 1;
        } else {
            new_content.push_str(&line);
        }
    }

    if hidden_count > 0 {
        new_content.push_str(&format!("\n... and {} more mods\n", hidden_count));
    }

    if total > 0 {
        if percentage {
            let percent = (enabled_count * 100) / total;
            new_content.push_str(&format!(
                "\ntotal: {}/{} ({}%)\n",
                enabled_count, total, percent
            ));
        } else {
            new_content.push_str(&format!("\ntotal: {}/{}\n", enabled_count, total));
        }
    }

    new_content
}
