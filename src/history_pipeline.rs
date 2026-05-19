use std::env;
use std::path::Path;

use crate::agents::LearningAgent;
use crate::domain::{BettingRules, RecommendationDecision};
use crate::history::PickHistoryEntry;
use crate::settlement::SettlementRecords;

pub struct HistoryState {
    entries: Vec<PickHistoryEntry>,
    output_path: Option<String>,
    settlements: Option<SettlementRecords>,
    settlement_updates: usize,
}

impl HistoryState {
    pub fn from_env() -> Result<Self, String> {
        Self::load(
            optional_env_path("BETTING_HISTORY_INPUT"),
            optional_env_path("BETTING_HISTORY_OUTPUT"),
            optional_env_path("BETTING_SETTLEMENTS_JSONL"),
        )
    }

    pub fn learning_agent(&self) -> LearningAgent {
        LearningAgent::from_entries(self.entries.clone())
    }

    pub fn write_recommendation(
        &mut self,
        recommendation: &RecommendationDecision,
        rules: &BettingRules,
    ) -> Result<(), String> {
        if self.output_path.is_none() {
            return Ok(());
        }

        crate::history::merge_recommendation_entries(&mut self.entries, recommendation, rules);
        self.apply_settlements();
        let output_path = self.output_path.as_deref().expect("checked output path");
        crate::history::write_entries(output_path, &self.entries)?;
        self.log_settlement_updates();

        Ok(())
    }

    fn load(
        input_path: Option<String>,
        output_path: Option<String>,
        settlements_path: Option<String>,
    ) -> Result<Self, String> {
        let mut entries = match input_path {
            Some(path) if Path::new(&path).exists() => crate::history::read_history_file(&path)?,
            _ => Vec::new(),
        };
        let settlements = settlements_path
            .as_deref()
            .map(SettlementRecords::read)
            .transpose()?;
        let mut state = Self {
            entries: Vec::new(),
            output_path,
            settlements,
            settlement_updates: 0,
        };

        state.entries.append(&mut entries);
        state.apply_settlements();
        Ok(state)
    }

    fn apply_settlements(&mut self) {
        if let Some(settlements) = &self.settlements {
            self.settlement_updates += settlements.apply_to(&mut self.entries);
        }
    }

    fn log_settlement_updates(&mut self) {
        if self.settlement_updates > 0 {
            eprintln!("settled {} pick history entries", self.settlement_updates);
            self.settlement_updates = 0;
        }
    }
}

fn optional_env_path(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::domain::{
        BetCandidate, EvaluatedCandidate, FootballContextAssessment, FootballContextCategory,
        FootballContextStatus, LearningAssessment, ProbabilityAssessment, ResearchAssessment,
        RiskAssessment, ValueAssessment,
    };
    use crate::history::{FootballContextStatusValue, HistoryContextCategory, ResultStatus};

    #[test]
    fn applies_settlements_to_learning_and_output_state() {
        let input_path = temp_path("history-input");
        let output_path = temp_path("history-output");
        let settlements_path = temp_path("settlements");
        let entries = (0..5)
            .map(|index| history_entry(index, ResultStatus::Pending))
            .collect::<Vec<_>>();
        crate::history::write_entries(&input_path, &entries).expect("write input history");
        fs::write(
            &settlements_path,
            (0..5).map(settlement_line).collect::<Vec<_>>().join("\n"),
        )
        .expect("write settlements");

        let mut state = HistoryState::load(
            Some(input_path.clone()),
            Some(output_path.clone()),
            Some(settlements_path.clone()),
        )
        .expect("load history state");
        let learning = state.learning_agent().assess(&candidate(0), &context());

        assert_eq!(learning.settled_samples, 5);
        assert_eq!(learning.wins, 5);
        assert!(learning.confidence_adjustment > 0.0);

        state
            .write_recommendation(
                &RecommendationDecision::NoBet {
                    reason: "no candidates".to_string(),
                    reviewed: Vec::new(),
                },
                &rules(),
            )
            .expect("write history");
        let output = crate::history::read_history_file(&output_path).expect("read output");

        assert_eq!(output.len(), 5);
        assert!(
            output
                .iter()
                .all(|entry| entry.result_status == ResultStatus::Win)
        );
        assert!(
            output
                .iter()
                .all(|entry| entry.settlement_source.as_deref() == Some("manual final score"))
        );
    }

    #[test]
    fn applies_same_run_settlement_after_recommendation_merge() {
        let output_path = temp_path("same-run-output");
        let settlements_path = temp_path("same-run-settlements");
        fs::write(&settlements_path, settlement_line(0)).expect("write settlements");
        let mut state = HistoryState::load(None, Some(output_path.clone()), Some(settlements_path))
            .expect("load history state");

        state
            .write_recommendation(
                &RecommendationDecision::BestAvailable {
                    reason: "ranked fallback".to_string(),
                    picks: vec![evaluated(0)],
                },
                &rules(),
            )
            .expect("write history");
        let output = crate::history::read_history_file(&output_path).expect("read output");

        assert_eq!(output.len(), 1);
        assert_eq!(output[0].result_status, ResultStatus::Win);
        assert_eq!(
            output[0].settlement_source.as_deref(),
            Some("manual final score")
        );
    }

    fn temp_path(label: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "betting-{label}-{}-{nanos}.jsonl",
                std::process::id()
            ))
            .display()
            .to_string()
    }

    fn rules() -> BettingRules {
        BettingRules {
            date: Some("2026-05-15".to_string()),
            ..BettingRules::default()
        }
    }

    fn settlement_line(index: usize) -> String {
        format!(
            r#"{{"report_date":"2026-05-15","candidate_id":"candidate-{index}","event":"Rosenborg - Brann","market":"Double chance","selection":"Rosenborg or draw","starts_at":"2026-05-15T1{index}:00:00+02:00","result_status":"win","settlement_source":"manual final score","settled_at":"2026-05-16T10:00:00Z"}}"#
        )
    }

    fn evaluated(index: usize) -> EvaluatedCandidate {
        EvaluatedCandidate {
            candidate: candidate(index),
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
            football_context: context(),
            learning: LearningAssessment::no_history(),
            score: 80.0,
            rejection_reasons: Vec::new(),
        }
    }

    fn candidate(index: usize) -> BetCandidate {
        BetCandidate {
            id: format!("candidate-{index}"),
            sport: "Football".to_string(),
            competition: "Eliteserien".to_string(),
            event: "Rosenborg - Brann".to_string(),
            market: "Double chance".to_string(),
            selection: "Rosenborg or draw".to_string(),
            norsk_tipping_odds: 1.22,
            model_probability: None,
            reference_odds: None,
            confidence: Some(0.75),
            starts_at: format!("2026-05-15T1{index}:00:00+02:00"),
            notes: String::new(),
        }
    }

    fn context() -> FootballContextAssessment {
        FootballContextAssessment {
            matched_pages: 0,
            categories: vec![FootballContextCategory {
                name: "Form".to_string(),
                status: FootballContextStatus::Unknown,
                evidence: Vec::new(),
            }],
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
            settlement_source: None,
            settlement_source_url: None,
            settled_at: None,
        }
    }
}
