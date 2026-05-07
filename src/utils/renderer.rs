use crate::emojis::{HARD_NO, NO, YES};
use crate::models::vote::VoteChoice;

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn generate_results_chart(votes: &[(i64, VoteChoice)]) -> String {
    let mut yes_count = 0;
    let mut no_count = 0;
    let mut hard_no_count = 0;

    for (_, choice) in votes {
        match choice {
            VoteChoice::Yes => yes_count += 1,
            VoteChoice::No => no_count += 1,
            VoteChoice::HardNo => hard_no_count += 1,
        }
    }

    let total_yes_score: f64 = f64::from(yes_count);
    let total_no_score: f64 = f64::from(hard_no_count).mul_add(1.5, f64::from(no_count));
    let total_votes = yes_count + no_count + hard_no_count;

    let yes_score = total_yes_score;
    let no_score = f64::from(no_count);
    let hard_no_score = f64::from(hard_no_count) * 1.5;

    let max_score = yes_score.max(no_score).max(hard_no_score).max(1.0_f64);
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

    let outcome_text = if total_yes_score > total_no_score {
        format!("**passed** (yes {total_yes_score} vs no {total_no_score})")
    } else if total_no_score > total_yes_score {
        format!("**failed** (yes {total_yes_score} vs no {total_no_score})")
    } else {
        format!("**tie** (score: {total_yes_score})")
    };

    format!(
        "{} {} | {} votes\n\
         {} {} | {} votes\n\
         {} {} | {} votes (weighted 1,5)\n\n\
         **outcome:**\n\
         {outcome_text}\n\n\
         *{total_votes} votes from {total_votes} users*",
        YES.text,
        make_bar(yes_score),
        yes_count,
        NO.text,
        make_bar(no_score),
        no_count,
        HARD_NO.text,
        make_bar(hard_no_score),
        hard_no_count
    )
}
