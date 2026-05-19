use std::collections::HashMap;
use std::env;
use std::path::Path;

use crate::domain::{
    BetCandidate, FootballContextAssessment, FootballContextStatus, LearningAssessment,
};
use crate::history::{FootballContextStatusValue, PickHistoryEntry, ResultStatus};

const MIN_BUCKET_SAMPLE: usize = 5;
const MAX_CONFIDENCE_ADJUSTMENT: f64 = 0.03;
const BASELINE_HIT_RATE: f64 = 0.80;

#[derive(Debug, Clone)]
pub struct LearningAgent {
    entries: Vec<PickHistoryEntry>,
    min_bucket_sample: usize,
}

impl LearningAgent {
    pub fn disabled() -> Self {
        Self {
            entries: Vec::new(),
            min_bucket_sample: MIN_BUCKET_SAMPLE,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let Ok(path) = env::var("BETTING_HISTORY_INPUT") else {
            return Ok(Self::disabled());
        };
        if path.trim().is_empty() || !Path::new(&path).exists() {
            return Ok(Self::disabled());
        }

        let mut entries = crate::history::read_history_file(&path)?;
        if let Ok(settlements_path) = env::var("BETTING_SETTLEMENTS_JSONL") {
            crate::settlement::apply_settlement_file(&mut entries, &settlements_path)?;
        }
        Ok(Self {
            entries,
            min_bucket_sample: MIN_BUCKET_SAMPLE,
        })
    }

    pub fn assess(
        &self,
        candidate: &BetCandidate,
        football_context: &FootballContextAssessment,
    ) -> LearningAssessment {
        let settled = self.settled_entries();
        if settled.is_empty() {
            return LearningAssessment::no_history();
        }

        let buckets = candidate_buckets(candidate, football_context);
        let bucket_counts = build_bucket_counts(&settled);
        let Some((bucket, counts)) = buckets
            .iter()
            .filter_map(|bucket| bucket_counts.get(bucket).map(|counts| (bucket, counts)))
            .find(|(_, counts)| counts.total() >= self.min_bucket_sample)
        else {
            let best_sample = buckets
                .iter()
                .filter_map(|bucket| bucket_counts.get(bucket).map(BucketCounts::total))
                .max()
                .unwrap_or(0);
            return LearningAssessment {
                bucket: None,
                settled_samples: best_sample,
                wins: 0,
                losses: 0,
                hit_rate: None,
                confidence_adjustment: 0.0,
                notes: vec![format!(
                    "history: insufficient similar settled picks ({best_sample}/{})",
                    self.min_bucket_sample
                )],
            };
        };

        let hit_rate = counts.wins as f64 / counts.total() as f64;
        let confidence_adjustment = ((hit_rate - BASELINE_HIT_RATE) * 0.10)
            .clamp(-MAX_CONFIDENCE_ADJUSTMENT, MAX_CONFIDENCE_ADJUSTMENT);

        LearningAssessment {
            bucket: Some(bucket.clone()),
            settled_samples: counts.total(),
            wins: counts.wins,
            losses: counts.losses,
            hit_rate: Some(hit_rate),
            confidence_adjustment,
            notes: vec![format!(
                "history: {bucket} {} settled picks, {:.0}% hit rate, {:+.2} pp confidence",
                counts.total(),
                hit_rate * 100.0,
                confidence_adjustment * 100.0
            )],
        }
    }

    fn settled_entries(&self) -> Vec<&PickHistoryEntry> {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.result_status, ResultStatus::Win | ResultStatus::Loss))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BucketCounts {
    wins: usize,
    losses: usize,
}

impl BucketCounts {
    fn add(&mut self, result_status: ResultStatus) {
        match result_status {
            ResultStatus::Win => self.wins += 1,
            ResultStatus::Loss => self.losses += 1,
            ResultStatus::Pending | ResultStatus::Void | ResultStatus::Unknown => {}
        }
    }

    fn total(&self) -> usize {
        self.wins + self.losses
    }
}

fn build_bucket_counts(entries: &[&PickHistoryEntry]) -> HashMap<String, BucketCounts> {
    let mut counts = HashMap::new();
    for entry in entries {
        for bucket in history_buckets(entry) {
            counts
                .entry(bucket)
                .or_insert_with(BucketCounts::default)
                .add(entry.result_status);
        }
    }
    counts
}

fn candidate_buckets(
    candidate: &BetCandidate,
    football_context: &FootballContextAssessment,
) -> Vec<String> {
    let market = market_type(&candidate.market, &candidate.selection);
    let odds = odds_bucket(candidate.norsk_tipping_odds);
    let selection = selection_type(&candidate.event, &candidate.selection);
    let warnings = warning_bucket(
        football_context
            .categories
            .iter()
            .filter(|category| category.status == FootballContextStatus::Warning)
            .map(|category| category.name.as_str()),
    );
    let competition = normalize_key(&candidate.competition);

    vec![
        format!(
            "competition={competition}|market={market}|odds={odds}|selection={selection}|warnings={warnings}"
        ),
        format!("market={market}|odds={odds}|selection={selection}|warnings={warnings}"),
        format!("market={market}|odds={odds}|selection={selection}"),
        format!("market={market}|odds={odds}"),
        format!("competition={competition}"),
    ]
}

fn history_buckets(entry: &PickHistoryEntry) -> Vec<String> {
    let market = market_type(&entry.market, &entry.selection);
    let odds = odds_bucket(entry.norsk_tipping_odds);
    let selection = selection_type(&entry.event, &entry.selection);
    let warnings = warning_bucket(
        entry
            .football_context
            .iter()
            .filter(|category| category.status == FootballContextStatusValue::Warning)
            .map(|category| category.name.as_str()),
    );
    let competition = normalize_key(&entry.competition);

    vec![
        format!(
            "competition={competition}|market={market}|odds={odds}|selection={selection}|warnings={warnings}"
        ),
        format!("market={market}|odds={odds}|selection={selection}|warnings={warnings}"),
        format!("market={market}|odds={odds}|selection={selection}"),
        format!("market={market}|odds={odds}"),
        format!("competition={competition}"),
    ]
}

fn market_type(market: &str, selection: &str) -> String {
    let combined = format!("{} {}", market, selection).to_ascii_lowercase();
    if combined.contains("double chance") {
        "double_chance".to_string()
    } else if combined.contains("over") || combined.contains("under") {
        "totals".to_string()
    } else if combined.contains("draw") {
        "draw_related".to_string()
    } else if combined.contains("winner") || combined.contains("match") {
        "match_winner".to_string()
    } else {
        normalize_key(market)
    }
}

fn odds_bucket(odds: f64) -> String {
    match odds {
        value if value < 1.10 => "below_band",
        value if value < 1.15 => "1.10-1.14",
        value if value < 1.20 => "1.15-1.19",
        value if value < 1.25 => "1.20-1.24",
        value if value <= 1.30 => "1.25-1.30",
        value if value <= 1.35 => "1.31-1.35_slack",
        _ => "above_band",
    }
    .to_string()
}

fn selection_type(event: &str, selection: &str) -> String {
    let normalized_selection = normalize_key(selection);
    if normalized_selection.contains("draw") {
        return "draw_related".to_string();
    }
    if normalized_selection.starts_with("over") {
        return "over".to_string();
    }
    if normalized_selection.starts_with("under") {
        return "under".to_string();
    }

    let teams = event
        .split(['-', '–'])
        .map(normalize_key)
        .collect::<Vec<_>>();
    if teams.len() >= 2 {
        if normalized_selection.contains(&teams[0]) {
            return "home".to_string();
        }
        if normalized_selection.contains(&teams[1]) {
            return "away".to_string();
        }
    }

    "other".to_string()
}

fn warning_bucket<'a>(warnings: impl Iterator<Item = &'a str>) -> String {
    let mut values = warnings.map(normalize_key).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join("+")
    }
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FootballContextCategory, FootballContextStatus};
    use crate::history::HistoryContextCategory;

    #[test]
    fn returns_no_history_note_without_settled_entries() {
        let learning = LearningAgent::disabled().assess(&candidate(), &context(Vec::new()));

        assert_eq!(learning.confidence_adjustment, 0.0);
        assert!(learning.notes[0].contains("no settled"));
    }

    #[test]
    fn applies_positive_bucket_adjustment() {
        let agent = agent_with_results(5, 0);
        let learning = agent.assess(&candidate(), &context(Vec::new()));

        assert_eq!(learning.settled_samples, 5);
        assert_eq!(learning.wins, 5);
        assert!(learning.confidence_adjustment > 0.0);
        assert!(learning.notes[0].contains("100% hit rate"));
    }

    #[test]
    fn applies_negative_bucket_adjustment() {
        let agent = agent_with_results(1, 4);
        let learning = agent.assess(&candidate(), &context(Vec::new()));

        assert_eq!(learning.settled_samples, 5);
        assert!(learning.confidence_adjustment < 0.0);
    }

    #[test]
    fn requires_minimum_sample_size() {
        let agent = agent_with_results(2, 1);
        let learning = agent.assess(&candidate(), &context(Vec::new()));

        assert_eq!(learning.confidence_adjustment, 0.0);
        assert!(learning.notes[0].contains("insufficient"));
    }

    fn agent_with_results(wins: usize, losses: usize) -> LearningAgent {
        let mut entries = Vec::new();
        for index in 0..wins {
            entries.push(history_entry(index, ResultStatus::Win));
        }
        for index in wins..(wins + losses) {
            entries.push(history_entry(index, ResultStatus::Loss));
        }
        LearningAgent {
            entries,
            min_bucket_sample: MIN_BUCKET_SAMPLE,
        }
    }

    fn candidate() -> BetCandidate {
        BetCandidate {
            id: "candidate".to_string(),
            sport: "Football".to_string(),
            competition: "Eliteserien".to_string(),
            event: "Rosenborg - Brann".to_string(),
            market: "Double chance".to_string(),
            selection: "Rosenborg or draw".to_string(),
            norsk_tipping_odds: 1.22,
            model_probability: None,
            reference_odds: None,
            confidence: Some(0.75),
            starts_at: "2026-05-15T18:00:00+02:00".to_string(),
            notes: String::new(),
        }
    }

    fn context(warning_names: Vec<&str>) -> FootballContextAssessment {
        FootballContextAssessment {
            matched_pages: 0,
            categories: warning_names
                .into_iter()
                .map(|name| FootballContextCategory {
                    name: name.to_string(),
                    status: FootballContextStatus::Warning,
                    evidence: Vec::new(),
                })
                .collect(),
            confidence_adjustment: 0.0,
            notes: Vec::new(),
        }
    }

    fn history_entry(index: usize, result_status: ResultStatus) -> PickHistoryEntry {
        PickHistoryEntry {
            report_date: "2026-05-15".to_string(),
            rank: index + 1,
            candidate_id: format!("candidate-{index}"),
            sport: "Football".to_string(),
            competition: "Eliteserien".to_string(),
            event: "Rosenborg - Brann".to_string(),
            market: "Double chance".to_string(),
            selection: "Rosenborg or draw".to_string(),
            starts_at: format!("2026-05-15T1{index}:00:00+02:00"),
            norsk_tipping_odds: 1.22,
            score: 80.0,
            confidence: 0.75,
            strict_status: "pass".to_string(),
            rejection_reasons: Vec::new(),
            football_context: vec![HistoryContextCategory {
                name: "Form".to_string(),
                status: FootballContextStatusValue::Unknown,
                evidence: Vec::new(),
            }],
            result_status,
            settlement_source: Some("test".to_string()),
            settlement_source_url: None,
            settled_at: Some("2026-05-16T10:00:00Z".to_string()),
        }
    }
}
