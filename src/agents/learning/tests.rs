use super::*;
use crate::domain::{FootballContextCategory, FootballContextStatus};
use crate::history::HistoryContextCategory;

#[test]
fn returns_no_history_note_without_settled_entries() {
    let learning =
        LearningAgent::from_entries(Vec::new()).assess(&candidate(), &context(Vec::new()));

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
    LearningAgent::from_entries(entries)
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
