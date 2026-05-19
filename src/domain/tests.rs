use super::*;

#[test]
fn football_scope_accepts_norwegian_and_english_names() {
    let scope = SportScope::Football;

    assert!(scope.allows_sport("Fotball"));
    assert!(scope.allows_sport("Football"));
    assert!(scope.allows_sport("Soccer"));
    assert!(scope.allows_sport("soccer - england"));
}

#[test]
fn football_scope_rejects_other_sports_and_football_variants() {
    let scope = SportScope::Football;

    assert!(!scope.allows_sport("Tennis"));
    assert!(!scope.allows_sport("Ishockey"));
    assert!(!scope.allows_sport("American Football"));
    assert!(!scope.allows_sport("Amerikansk fotball"));
}

#[test]
fn rules_filter_csv_candidates_to_football_scope() {
    let football = test_candidate("football", "Football");
    let tennis = test_candidate("tennis", "Tennis");

    let filtered = BettingRules::default()
        .filter_by_sport_scope(vec![football.clone(), tennis])
        .expect("football candidate remains");

    assert_eq!(filtered, vec![football]);
}

fn test_candidate(id: &str, sport: &str) -> BetCandidate {
    BetCandidate {
        id: id.to_string(),
        sport: sport.to_string(),
        competition: "Competition".to_string(),
        event: "Home - Away".to_string(),
        market: "Main market".to_string(),
        selection: "Home".to_string(),
        norsk_tipping_odds: 1.20,
        model_probability: None,
        reference_odds: None,
        confidence: Some(0.80),
        starts_at: "2026-05-15T18:00:00+02:00".to_string(),
        notes: String::new(),
    }
}
