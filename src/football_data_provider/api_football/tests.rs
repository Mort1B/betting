use std::collections::HashMap;

use crate::domain::{BetCandidate, FootballContextStatus};
use crate::football_context::assess_football_context;

use super::context::{
    append_fixture_note, append_form_notes, append_injury_notes, match_candidates,
};
use super::{ApiFixture, ApiFootballOptions, ApiFootballProvider, ProviderStats, parse_envelope};

#[test]
fn enriches_candidate_notes_from_fixture_injuries_and_form() {
    let fixtures =
        parse_envelope::<ApiFixture>(include_str!("../../../fixtures/api_football_fixtures.json"))
            .expect("fixture JSON");
    let injuries = parse_envelope(include_str!("../../../fixtures/api_football_injuries.json"))
        .expect("injury JSON");
    let home_form = parse_envelope::<ApiFixture>(include_str!(
        "../../../fixtures/api_football_team_331_form.json"
    ))
    .expect("home form JSON");
    let away_form = parse_envelope::<ApiFixture>(include_str!(
        "../../../fixtures/api_football_team_332_form.json"
    ))
    .expect("away form JSON");
    let mut candidates = vec![candidate()];

    let matches = match_candidates(&candidates, &fixtures);
    assert_eq!(matches.len(), 1);

    let fixture = &fixtures[matches[0].fixture_index];
    append_fixture_note(&mut candidates[0], fixture);
    append_injury_notes(&mut candidates[0], fixture, &injuries);
    append_form_notes(
        &mut candidates[0],
        fixture,
        &HashMap::from([(331_u64, home_form), (332_u64, away_form)]),
    );

    assert!(candidates[0].notes.contains("API-Football fixture matched"));
    assert!(candidates[0].notes.contains("Hamstring injury"));
    assert!(candidates[0].notes.contains("Rosenborg recent form WWW"));
    assert!(candidates[0].notes.contains("Brann recent form LLL"));
    let context = assess_football_context(&candidates[0], None);
    assert!(
        context
            .categories
            .iter()
            .any(|category| category.name == "Injuries/suspensions"
                && category.status == FootballContextStatus::Warning)
    );
    assert!(
        context
            .categories
            .iter()
            .any(|category| category.name == "Schedule/travel"
                && category.status == FootballContextStatus::Warning)
    );
}

#[test]
fn summarizes_request_counts_without_secret_values() {
    let stats = ProviderStats {
        fixture_requests: 1,
        fixture_success: 1,
        injury_requests: 1,
        injury_success: 1,
        form_requests: 2,
        form_success: 2,
    };

    let summary = stats.summary(1, 3);

    assert!(summary.contains("API-Football: fixture requests 1/1"));
    assert!(summary.contains("matched 1/3 candidate"));
    assert!(!summary.contains("key"));
}

#[test]
fn redacts_api_key_from_errors() {
    let provider = ApiFootballProvider::new(ApiFootballOptions::new("secret-key".to_string()));

    let error = provider.public_error("request failed for secret-key through x-apisports-key");

    assert!(error.contains("<redacted>"));
    assert!(error.contains("API-Football key"));
    assert!(!error.contains("secret-key"));
}

fn candidate() -> BetCandidate {
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
        confidence: Some(0.80),
        starts_at: "2026-05-15T18:00:00+02:00".to_string(),
        notes: String::new(),
    }
}
