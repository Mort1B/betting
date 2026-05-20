use super::*;
use crate::research::{ResearchPage, ResearchSignal};

#[test]
fn leaves_unmatched_research_unknown_without_boost() {
    let digest = ResearchDigest {
        pages: vec![page(
            "preview",
            "Tennis final",
            "Good form and full squad are mentioned for another event.",
        )],
    };

    let assessment = assess_football_context(&candidate(""), Some(&digest));

    assert_eq!(assessment.matched_pages, 0);
    assert_eq!(assessment.confidence_adjustment, 0.0);
    assert!(
        assessment
            .categories
            .iter()
            .all(|category| category.status == FootballContextStatus::Unknown)
    );
}

#[test]
fn ignores_generic_research_without_event_terms() {
    let digest = ResearchDigest {
        pages: vec![page(
            "SportyTrader football tips",
            "Football tips",
            "Premier League Europe tips mention over goals and draw markets.",
        )],
    };

    let assessment = assess_football_context(&candidate(""), Some(&digest));

    assert_eq!(assessment.matched_pages, 0);
    assert!(
        assessment
            .categories
            .iter()
            .all(|category| category.status == FootballContextStatus::Unknown)
    );
}

#[test]
fn downgrades_candidate_specific_warning_context() {
    let digest = ResearchDigest {
        pages: vec![page(
            "preview",
            "Rosenborg Brann preview",
            "Rosenborg - Brann has injury news, short rest and could be a dead rubber.",
        )],
    };

    let assessment = assess_football_context(&candidate(""), Some(&digest));

    assert_eq!(assessment.matched_pages, 1);
    assert!(assessment.confidence_adjustment < 0.0);
    assert!(assessment.warning_count() >= 3);
}

#[test]
fn ignores_warning_terms_far_from_candidate_context() {
    let digest = ResearchDigest {
        pages: vec![page(
            "preview",
            "Rosenborg Brann preview",
            "Rosenborg - Brann has a stable preview. analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis Another unrelated match has injury news and short rest.",
        )],
    };

    let assessment = assess_football_context(&candidate(""), Some(&digest));

    assert_eq!(assessment.matched_pages, 1);
    assert_eq!(assessment.warning_count(), 0);
}

#[test]
fn uses_candidate_notes_as_supplied_context() {
    let assessment = assess_football_context(&candidate("strong form"), None);

    assert_eq!(assessment.matched_pages, 0);
    assert_eq!(assessment.confidence_adjustment, 0.0);
    assert!(
        assessment
            .categories
            .iter()
            .any(|category| category.status == FootballContextStatus::Positive)
    );
}

#[test]
fn uses_reference_market_notes_for_market_context() {
    let assessment = assess_football_context(&candidate("market agreement tight"), None);
    let market = assessment
        .categories
        .iter()
        .find(|category| category.name == "Market context")
        .expect("market context category");

    assert_eq!(market.status, FootballContextStatus::Positive);
}

#[test]
fn uses_specific_european_table_context_from_candidate_notes() {
    let assessment = assess_football_context(
        &candidate("API-Football motivation: Rosenborg europe place"),
        None,
    );
    let motivation = assessment
        .categories
        .iter()
        .find(|category| category.name == "Motivation")
        .expect("motivation category");

    assert_eq!(motivation.status, FootballContextStatus::Positive);
}

fn candidate(notes: &str) -> BetCandidate {
    BetCandidate {
        id: "c1".to_string(),
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
        notes: notes.to_string(),
    }
}

fn page(source_name: &str, title: &str, text: &str) -> ResearchPage {
    ResearchPage::new(
        source_name.to_string(),
        "https://example.test".to_string(),
        title.to_string(),
        text.to_string(),
        vec![ResearchSignal::Warning("injury".to_string())],
        None,
    )
}
