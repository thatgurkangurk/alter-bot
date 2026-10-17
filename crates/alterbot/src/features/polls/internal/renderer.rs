use crate::models::{poll_option, vote};
use std::collections::HashMap;

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn generate_results_chart(options: &[poll_option::Model], votes: &[vote::Model]) -> String {
    let mut vote_counts: HashMap<uuid::Uuid, usize> = HashMap::new();
    for v in votes {
        *vote_counts.entry(v.option_id).or_insert(0) += 1;
    }

    let total_votes = votes.len();
    let mut option_results: Vec<(&poll_option::Model, usize, f64)> =
        Vec::with_capacity(options.len());
    let mut max_score = 0.0_f64;

    for opt in options {
        let raw_count = *vote_counts.get(&opt.id).unwrap_or(&0);
        let score = f64::from(raw_count as u32) * opt.weight;

        if score > max_score {
            max_score = score;
        }

        option_results.push((opt, raw_count, score));
    }

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

    let total_score: f64 = option_results.iter().map(|(_, _, score)| *score).sum();
    let mut lines = Vec::new();

    for (index, (opt, count, score)) in option_results.iter().enumerate() {
        let index_u32 = u32::try_from(index).unwrap_or(0);

        let prefix = if index_u32 < 26 {
            char::from_u32(0x1F1E6 + index_u32).map_or_else(|| "🔹".to_string(), |c| c.to_string())
        } else {
            "🔹".to_string()
        };

        let weight_text = format!(" (weighted {:.2})", opt.weight);

        let percentage = if total_score > 0.0 {
            (*score / total_score) * 100.0
        } else {
            0.0
        };

        lines.push(format!(
            "{} {} | {} votes ({:.1}% score){}",
            prefix,
            make_bar(*score),
            count,
            percentage,
            weight_text
        ));
    }

    const EPSILON: f64 = 1e-9;
    let outcome_text = if option_results.is_empty() {
        "**no options configured**".to_string()
    } else {
        let top_score = option_results
            .iter()
            .map(|(_, _, s)| *s)
            .fold(0.0_f64, f64::max);

        let winners: Vec<_> = option_results
            .iter()
            .filter(|(_, _, s)| (s - top_score).abs() < EPSILON)
            .collect();

        if top_score <= EPSILON {
            "**no votes cast**".to_string()
        } else if winners.len() > 1 {
            format!("**tie** (highest score: {})", f(top_score))
        } else {
            format!(
                "**winner:** {} (score: {})",
                winners[0].0.label,
                f(winners[0].2)
            )
        }
    };

    lines.push(String::new());
    lines.push("**outcome:**".to_string());
    lines.push(outcome_text);
    lines.push(String::new());
    lines.push(format!("*{total_votes} votes from {total_votes} users*"));

    lines.join("\n")
}
