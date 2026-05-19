use std::fs;

use serde::{Deserialize, Serialize};

use crate::domain::{
    BettingRules, EvaluatedCandidate, FootballContextStatus, RecommendationDecision,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PickHistoryEntry {
    pub(crate) report_date: String,
    pub(crate) rank: usize,
    pub(crate) candidate_id: String,
    pub(crate) sport: String,
    pub(crate) competition: String,
    pub(crate) event: String,
    pub(crate) market: String,
    pub(crate) selection: String,
    pub(crate) starts_at: String,
    pub(crate) norsk_tipping_odds: f64,
    pub(crate) score: f64,
    pub(crate) confidence: f64,
    pub(crate) strict_status: String,
    pub(crate) rejection_reasons: Vec<String>,
    pub(crate) football_context: Vec<HistoryContextCategory>,
    pub(crate) result_status: ResultStatus,
    pub(crate) settlement_source: Option<String>,
    pub(crate) settlement_source_url: Option<String>,
    pub(crate) settled_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HistoryContextCategory {
    pub(crate) name: String,
    pub(crate) status: FootballContextStatusValue,
    pub(crate) evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FootballContextStatusValue {
    Positive,
    Warning,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResultStatus {
    Pending,
    Win,
    Loss,
    Void,
    Unknown,
}

impl ResultStatus {
    pub(crate) fn is_settled(self) -> bool {
        matches!(self, Self::Win | Self::Loss | Self::Void)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HistoryKey {
    pub(crate) report_date: String,
    pub(crate) event: String,
    pub(crate) market: String,
    pub(crate) selection: String,
    pub(crate) starts_at: String,
}

pub(crate) fn merge_recommendation_entries(
    entries: &mut Vec<PickHistoryEntry>,
    recommendation: &RecommendationDecision,
    rules: &BettingRules,
) {
    merge_entries(entries, current_entries(recommendation, rules));
}

fn current_entries(
    recommendation: &RecommendationDecision,
    rules: &BettingRules,
) -> Vec<PickHistoryEntry> {
    let report_date = rules.date.clone().unwrap_or_else(|| "unknown".to_string());
    selected_candidates(recommendation)
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| entry_from_candidate(&report_date, index + 1, candidate))
        .collect()
}

fn selected_candidates(recommendation: &RecommendationDecision) -> Vec<&EvaluatedCandidate> {
    match recommendation {
        RecommendationDecision::Bet {
            selected,
            alternatives,
        } => std::iter::once(selected.as_ref())
            .chain(alternatives.iter())
            .collect(),
        RecommendationDecision::BestAvailable { picks, .. } => picks.iter().collect(),
        RecommendationDecision::NoBet { .. } => Vec::new(),
    }
}

fn entry_from_candidate(
    report_date: &str,
    rank: usize,
    candidate: &EvaluatedCandidate,
) -> PickHistoryEntry {
    PickHistoryEntry {
        report_date: report_date.to_string(),
        rank,
        candidate_id: candidate.candidate.id.clone(),
        sport: candidate.candidate.sport.clone(),
        competition: candidate.candidate.competition.clone(),
        event: candidate.candidate.event.clone(),
        market: candidate.candidate.market.clone(),
        selection: candidate.candidate.selection.clone(),
        starts_at: candidate.candidate.starts_at.clone(),
        norsk_tipping_odds: candidate.candidate.norsk_tipping_odds,
        score: candidate.score,
        confidence: candidate.risk.confidence,
        strict_status: if candidate.is_bettable() {
            "pass".to_string()
        } else {
            "fallback".to_string()
        },
        rejection_reasons: candidate.rejection_reasons.clone(),
        football_context: candidate
            .football_context
            .categories
            .iter()
            .map(|category| HistoryContextCategory {
                name: category.name.clone(),
                status: status_value(category.status),
                evidence: category.evidence.clone(),
            })
            .collect(),
        result_status: ResultStatus::Pending,
        settlement_source: None,
        settlement_source_url: None,
        settled_at: None,
    }
}

fn merge_entries(entries: &mut Vec<PickHistoryEntry>, new_entries: Vec<PickHistoryEntry>) {
    for mut new_entry in new_entries {
        if let Some(existing) = entries
            .iter_mut()
            .find(|entry| entry.key() == new_entry.key())
        {
            if existing.result_status != ResultStatus::Pending {
                new_entry.result_status = existing.result_status;
                new_entry.settlement_source = existing.settlement_source.clone();
                new_entry.settlement_source_url = existing.settlement_source_url.clone();
                new_entry.settled_at = existing.settled_at.clone();
            }
            *existing = new_entry;
        } else {
            entries.push(new_entry);
        }
    }
}

pub(crate) fn read_history_file(path: &str) -> Result<Vec<PickHistoryEntry>, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    let mut entries = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry = serde_json::from_str::<PickHistoryEntry>(trimmed)
            .map_err(|error| format!("{path}: line {}: {error}", index + 1))?;
        entries.push(entry);
    }

    Ok(entries)
}

pub(crate) fn write_entries(path: &str, entries: &[PickHistoryEntry]) -> Result<(), String> {
    let mut content = String::new();
    for entry in entries {
        let line = serde_json::to_string(entry)
            .map_err(|error| format!("failed to serialize history entry: {error}"))?;
        content.push_str(&line);
        content.push('\n');
    }
    fs::write(path, content).map_err(|error| format!("{path}: {error}"))
}

fn status_value(status: FootballContextStatus) -> FootballContextStatusValue {
    match status {
        FootballContextStatus::Positive => FootballContextStatusValue::Positive,
        FootballContextStatus::Warning => FootballContextStatusValue::Warning,
        FootballContextStatus::Unknown => FootballContextStatusValue::Unknown,
    }
}

impl PickHistoryEntry {
    pub(crate) fn key(&self) -> HistoryKey {
        HistoryKey {
            report_date: self.report_date.clone(),
            event: self.event.clone(),
            market: self.market.clone(),
            selection: self.selection.clone(),
            starts_at: self.starts_at.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        BetCandidate, FootballContextAssessment, FootballContextCategory, LearningAssessment,
        ProbabilityAssessment, ResearchAssessment, RiskAssessment, ValueAssessment,
    };

    #[test]
    fn merges_rerun_without_duplicate_entries() {
        let mut entries = vec![entry("2026-05-15", 1, ResultStatus::Pending)];
        merge_entries(
            &mut entries,
            vec![entry("2026-05-15", 1, ResultStatus::Pending)],
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].rank, 1);
    }

    #[test]
    fn preserves_settled_result_on_rerun() {
        let mut settled = entry("2026-05-15", 1, ResultStatus::Win);
        settled.settlement_source = Some("verified".to_string());
        settled.settled_at = Some("2026-05-16T10:00:00Z".to_string());
        let mut entries = vec![settled];

        merge_entries(
            &mut entries,
            vec![entry("2026-05-15", 1, ResultStatus::Pending)],
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].result_status, ResultStatus::Win);
        assert_eq!(entries[0].settlement_source.as_deref(), Some("verified"));
    }

    #[test]
    fn builds_entries_from_selected_candidates() {
        let recommendation = RecommendationDecision::Bet {
            selected: Box::new(evaluated("one")),
            alternatives: vec![evaluated("two")],
        };
        let rules = BettingRules {
            date: Some("2026-05-15".to_string()),
            ..BettingRules::default()
        };

        let entries = current_entries(&recommendation, &rules);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].report_date, "2026-05-15");
        assert_eq!(entries[1].rank, 2);
        assert_eq!(entries[0].result_status, ResultStatus::Pending);
    }

    fn entry(report_date: &str, rank: usize, result_status: ResultStatus) -> PickHistoryEntry {
        PickHistoryEntry {
            report_date: report_date.to_string(),
            rank,
            candidate_id: "candidate".to_string(),
            sport: "Football".to_string(),
            competition: "Eliteserien".to_string(),
            event: "Rosenborg - Brann".to_string(),
            market: "Double chance".to_string(),
            selection: "Rosenborg or draw".to_string(),
            starts_at: "2026-05-15T18:00:00+02:00".to_string(),
            norsk_tipping_odds: 1.22,
            score: 80.0,
            confidence: 0.75,
            strict_status: "pass".to_string(),
            rejection_reasons: Vec::new(),
            football_context: Vec::new(),
            result_status,
            settlement_source: None,
            settlement_source_url: None,
            settled_at: None,
        }
    }

    fn evaluated(id: &str) -> EvaluatedCandidate {
        EvaluatedCandidate {
            candidate: BetCandidate {
                id: id.to_string(),
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
            },
            probability: ProbabilityAssessment {
                estimated_probability: 0.82,
                implied_probability: 0.81,
                sources: vec!["test".to_string()],
                notes: Vec::new(),
            },
            value: ValueAssessment {
                expected_value: 0.01,
                edge: 0.01,
                value_notes: Vec::new(),
            },
            risk: RiskAssessment {
                confidence: 0.75,
                flags: Vec::new(),
                notes: Vec::new(),
            },
            research: ResearchAssessment::empty(),
            football_context: FootballContextAssessment {
                matched_pages: 0,
                categories: vec![FootballContextCategory {
                    name: "Form".to_string(),
                    status: FootballContextStatus::Unknown,
                    evidence: Vec::new(),
                }],
                confidence_adjustment: 0.0,
                notes: Vec::new(),
            },
            learning: LearningAssessment::no_history(),
            score: 80.0,
            rejection_reasons: Vec::new(),
        }
    }
}
