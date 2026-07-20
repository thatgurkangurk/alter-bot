use std::collections::HashMap;
use std::fmt::Write;

pub fn format_mod_statuses(results: &HashMap<String, bool>, percentage: bool) -> String {
    let mut mods: Vec<_> = results.keys().collect();
    mods.sort();

    let total = mods.len();
    let mut enabled_count = 0;
    for status in results.values() {
        if *status {
            enabled_count += 1;
        }
    }

    let mut new_content = String::new();
    let mut hidden_count = 0;

    let max_len = 3900;

    for mod_name in &mods {
        let status = results.get(*mod_name).unwrap_or(&false);
        let mark = if *status { "✅" } else { "❌" };
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
        if percentage {
            let percent = (enabled_count * 100_usize).checked_div(total).unwrap_or(0);
            let _ = write!(
                new_content,
                "\ntotal: {enabled_count}/{total} ({percent}%)\n"
            );
        } else {
            let _ = write!(new_content, "\ntotal: {enabled_count}/{total}\n");
        }
    }

    new_content
}
