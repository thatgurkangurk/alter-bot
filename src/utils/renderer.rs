use crate::models::vote::VoteChoice;

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn generate_results_chart(votes: &[(i64, VoteChoice)]) -> String {
    let mut yes_count = 0;
    let mut no_count = 0;
    let mut hard_no_count = 0;

    let mut yes_score: f64 = 0.0;
    let mut no_score: f64 = 0.0;
    let mut hard_no_score: f64 = 0.0;

    for (_, choice) in votes {
        match choice {
            VoteChoice::Yes => {
                yes_count += 1;
                yes_score += 1.0;
            }
            VoteChoice::No => {
                no_count += 1;
                no_score += 1.0;
            }
            VoteChoice::HardNo => {
                hard_no_count += 1;
                hard_no_score += 1.5;
            }
        }
    }

    let total_votes = yes_count + no_count + hard_no_count;
    let total_score = yes_score + no_score + hard_no_score;

    let max_score = yes_score.max(no_score).max(hard_no_score).max(1.0_f64);
    let max_bar_len: f64 = 20.0;

    let make_bar = |score: f64| -> String {
        if score == 0.0 {
            return String::new();
        }

        let total_half_blocks = ((score / max_score) * max_bar_len * 2.0).round() as usize;
        let full_blocks = total_half_blocks / 2;
        let has_half_block = total_half_blocks % 2 != 0;

        let mut bar = "█".repeat(full_blocks);
        if has_half_block {
            bar.push('▌');
        }
        bar
    };

    let calc_pct = |score: f64| -> f64 {
        if total_score == 0.0 {
            0.0
        } else {
            (score / total_score) * 100.0
        }
    };

    let yes_bars = make_bar(yes_score);
    let no_bars = make_bar(no_score);
    let hard_no_bars = make_bar(hard_no_score);

    format!(
        "```text\nYes      | {yes_bars:<20} ({yes_pct:>5.1}%  {yes_count}/{total_votes})\nNo       | {no_bars:<20} ({no_pct:>5.1}%  {no_count}/{total_votes})\nHard no  | {hard_no_bars:<20} ({hard_no_pct:>5.1}%  {hard_no_count}/{total_votes})\n```",
        yes_bars = yes_bars,
        yes_pct = calc_pct(yes_score),
        yes_count = yes_count,
        no_bars = no_bars,
        no_pct = calc_pct(no_score),
        no_count = no_count,
        hard_no_bars = hard_no_bars,
        hard_no_pct = calc_pct(hard_no_score),
        hard_no_count = hard_no_count,
        total_votes = total_votes
    )
}
