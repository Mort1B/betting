use std::env;

use crate::domain::{BettingRules, RecommendationDecision};

pub fn write_history_from_env(
    recommendation: &RecommendationDecision,
    rules: &BettingRules,
) -> Result<(), String> {
    let Ok(history_output) = env::var("BETTING_HISTORY_OUTPUT") else {
        return Ok(());
    };
    let history_input = env::var("BETTING_HISTORY_INPUT").ok();
    crate::history::write_history_file(
        recommendation,
        rules,
        history_input.as_deref(),
        &history_output,
    )?;

    if let Ok(settlements_path) = env::var("BETTING_SETTLEMENTS_JSONL") {
        let updated = crate::settlement::settle_history_file(&history_output, &settlements_path)?;
        if updated > 0 {
            eprintln!("settled {updated} pick history entries");
        }
    }

    Ok(())
}
