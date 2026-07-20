use crate::models::{poll_option, vote};
use std::collections::HashMap;

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn generate_results_chart(options: &[poll_option::Model], votes: &[vote::Model]) -> String {
    let mut vote_counts: HashMap<uuid::Uuid, usize> = HashMap::new();
    for v in votes {
        *vote_counts.entry(v.option_id).or_insert(0) += 1;
    }

    let total_votes = votes.len();

    let mut option_results: Vec<(&poll_option::Model, usize, f64)> = Vec::new();
    let mut max_score = 1.0_f64; // to avoid divide-by-zero

    for opt in options {
        let raw_count = *vote_counts.get(&opt.id).unwrap_or(&0);
        let score = f64::from(raw_count as u32) * opt.weight;

        if score > max_score {
            max_score = score;
        }

        option_results.push((opt, raw_count, score));
    }

    option_results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let max_bar_len: f64 = 10.0;
    let make_bar = |score: f64| -> String {
        let filled_blocks = if max_score > 0.0 {
            ((score / max_score) * max_bar_len).round() as usize
        } else {
            0
        };
        let filled_blocks = filled_blocks.min(10);
        format!(
            "[{}{}]",
            "▓".repeat(filled_blocks),
            "░".repeat(10 - filled_blocks)
        )
    };

    let f = |n: f64| format!("{n:.1}").replace('.', ",");

    let mut lines = Vec::new();

    for (index, (opt, count, score)) in option_results.iter().enumerate() {
        let index_u32 = u32::try_from(index).unwrap_or(0);

        let prefix =
            char::from_u32(0x1F1E6 + index_u32).map_or_else(|| "🔹".to_string(), |c| c.to_string());

        let weight_text = format!(" (weighted {:.2})", opt.weight);

        lines.push(format!(
            "{} {} | {} votes{}",
            prefix,
            make_bar(*score),
            count,
            weight_text
        ));
    }

    let outcome_text = if option_results.is_empty() {
        "**no options configured**".to_string()
    } else if option_results.len() > 1
        && (option_results[0].2 - option_results[1].2).abs() < f64::EPSILON
    {
        format!("**tie** (highest score: {})", f(option_results[0].2))
    } else {
        format!(
            "**winner:** {} (score: {})",
            option_results[0].0.label,
            f(option_results[0].2)
        )
    };

    lines.push(String::new());
    lines.push("**outcome:**".to_string());
    lines.push(outcome_text);
    lines.push(String::new());
    lines.push(format!("*{total_votes} votes from {total_votes} users*"));

    lines.join("\n")
}
