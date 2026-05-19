use std::collections::HashMap;

use crate::domain::{
    BetCandidate, FootballContextAssessment, FootballContextStatus, LearningAssessment,
};
use crate::history::{FootballContextStatusValue, PickHistoryEntry, ResultStatus};

const MIN_BUCKET_SAMPLE: usize = 5;
const MAX_CONFIDENCE_ADJUSTMENT: f64 = 0.03;
const BASELINE_HIT_RATE: f64 = 0.80;

#[derive(Debug, Clone)]
pub struct LearningAgent {
    bucket_counts: HashMap<String, BucketCounts>,
    min_bucket_sample: usize,
}

impl LearningAgent {
    pub(crate) fn from_entries(entries: Vec<PickHistoryEntry>) -> Self {
        Self {
            bucket_counts: build_bucket_counts(&entries),
            min_bucket_sample: MIN_BUCKET_SAMPLE,
        }
    }

    pub fn assess(
        &self,
        candidate: &BetCandidate,
        football_context: &FootballContextAssessment,
    ) -> LearningAssessment {
        if self.bucket_counts.is_empty() {
            return LearningAssessment::no_history();
        }

        let buckets = candidate_buckets(candidate, football_context);
        let Some((bucket, counts)) = buckets
            .iter()
            .filter_map(|bucket| {
                self.bucket_counts
                    .get(bucket)
                    .map(|counts| (bucket, counts))
            })
            .find(|(_, counts)| counts.total() >= self.min_bucket_sample)
        else {
            let best_sample = buckets
                .iter()
                .filter_map(|bucket| self.bucket_counts.get(bucket).map(BucketCounts::total))
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

fn build_bucket_counts(entries: &[PickHistoryEntry]) -> HashMap<String, BucketCounts> {
    let mut counts = HashMap::new();
    for entry in entries
        .iter()
        .filter(|entry| matches!(entry.result_status, ResultStatus::Win | ResultStatus::Loss))
    {
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
mod tests;
