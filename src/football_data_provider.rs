mod api_football;

use crate::domain::{BetCandidate, BettingRules};

pub use api_football::ApiFootballOptions;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FootballDataOptions {
    pub api_football: Option<ApiFootballOptions>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FootballDataResult {
    pub candidates: Vec<BetCandidate>,
    pub provider_report_notes: Vec<String>,
}

pub trait FootballContextProvider {
    fn enrich_candidates(
        &self,
        candidates: Vec<BetCandidate>,
        rules: &BettingRules,
    ) -> FootballDataResult;
}

pub fn apply_football_data(
    candidates: Vec<BetCandidate>,
    rules: &BettingRules,
    options: &FootballDataOptions,
) -> FootballDataResult {
    if let Some(options) = &options.api_football {
        return api_football::ApiFootballProvider::new(options.clone())
            .enrich_candidates(candidates, rules);
    }

    FootballDataResult {
        candidates,
        provider_report_notes: Vec::new(),
    }
}

pub(crate) fn append_candidate_note(candidate: &mut BetCandidate, note: String) {
    if note.trim().is_empty() {
        return;
    }
    if candidate.notes.trim().is_empty() {
        candidate.notes = note;
    } else {
        candidate.notes = format!("{}; {note}", candidate.notes.trim());
    }
}
