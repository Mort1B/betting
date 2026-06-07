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

#[test]
fn treats_opponent_context_with_selection_direction() {
    let assessment = assess_football_context(
        &candidate(
            "API-Football form: Away recent form LLL; opponent vulnerable form; API-Football opponent absences: Away Defender",
        ),
        None,
    );
    let form = assessment
        .categories
        .iter()
        .find(|category| category.name == "Form")
        .expect("form category");
    let injuries = assessment
        .categories
        .iter()
        .find(|category| category.name == "Injuries/suspensions")
        .expect("injury category");

    assert_eq!(form.status, FootballContextStatus::Positive);
    assert_eq!(injuries.status, FootballContextStatus::Positive);
}

#[test]
fn clean_injury_phrase_does_not_become_warning() {
    let assessment = assess_football_context(&candidate("no fresh injury; full squad"), None);
    let injuries = assessment
        .categories
        .iter()
        .find(|category| category.name == "Injuries/suspensions")
        .expect("injury category");

    assert_eq!(injuries.status, FootballContextStatus::Positive);
    assert!(
        !injuries
            .evidence
            .iter()
            .any(|evidence| evidence.contains("warning injury"))
    );
}

#[test]
fn explains_unknown_api_context_when_fixture_does_not_match() {
    let assessment = assess_football_context(
        &candidate("API-Football fixture not matched: no provider fixture matched teams/start"),
        None,
    );

    let form = assessment
        .categories
        .iter()
        .find(|category| category.name == "Form")
        .expect("form category");
    let market = assessment
        .categories
        .iter()
        .find(|category| category.name == "Market context")
        .expect("market category");

    assert_eq!(form.status, FootballContextStatus::Unknown);
    assert!(
        form.evidence
            .iter()
            .any(|evidence| evidence.contains("fixture not matched"))
    );
    assert!(
        market
            .evidence
            .iter()
            .any(|evidence| evidence.contains("no candidate-level reference price matched"))
    );
}

#[test]
fn explains_unknown_api_context_when_coverage_is_missing() {
    let assessment = assess_football_context(
        &candidate(
            "API-Football fixture matched: Home vs Away; API-Football availability coverage not confirmed; API-Football table coverage unavailable; API-Football form checked: no completed recent fixture data",
        ),
        None,
    );

    let injuries = assessment
        .categories
        .iter()
        .find(|category| category.name == "Injuries/suspensions")
        .expect("injury category");
    let motivation = assessment
        .categories
        .iter()
        .find(|category| category.name == "Motivation")
        .expect("motivation category");

    assert_eq!(injuries.status, FootballContextStatus::Unknown);
    assert!(
        injuries
            .evidence
            .iter()
            .any(|evidence| evidence.contains("coverage not confirmed"))
    );
    assert!(
        motivation
            .evidence
            .iter()
            .any(|evidence| evidence.contains("coverage unavailable"))
    );
}

#[test]
fn explains_unknown_api_context_when_enrichment_cap_skips_match() {
    let assessment = assess_football_context(
        &candidate(
            "API-Football fixture matched but context enrichment skipped: matched fixture cap 1 reached",
        ),
        None,
    );

    let form = assessment
        .categories
        .iter()
        .find(|category| category.name == "Form")
        .expect("form category");
    let schedule = assessment
        .categories
        .iter()
        .find(|category| category.name == "Schedule/travel")
        .expect("schedule category");

    assert_eq!(form.status, FootballContextStatus::Unknown);
    assert!(
        form.evidence
            .iter()
            .any(|evidence| evidence.contains("skipped by cap"))
    );
    assert!(
        schedule
            .evidence
            .iter()
            .any(|evidence| evidence.contains("skipped by cap"))
    );
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
