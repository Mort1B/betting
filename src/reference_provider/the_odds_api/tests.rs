use super::time::iso_to_utc_minutes;
use super::*;

fn candidate() -> BetCandidate {
    BetCandidate {
        id: "nt-rosenborg-brann".to_string(),
        sport: "Football".to_string(),
        competition: "Eliteserien".to_string(),
        event: "Rosenborg - Brann".to_string(),
        market: "Main market".to_string(),
        selection: "Rosenborg".to_string(),
        norsk_tipping_odds: 1.22,
        model_probability: None,
        reference_odds: None,
        confidence: Some(0.72),
        starts_at: "2026-05-16T18:00:00.000+02:00".to_string(),
        notes: "live import".to_string(),
    }
}

#[test]
fn maps_fixture_h2h_odds_to_reference_rows() {
    let events =
        parse_events(include_str!("../../../fixtures/the_odds_api_h2h.json")).expect("fixture");
    let rows = reference_rows_from_events(&[candidate()], &events);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].candidate_id.as_deref(), Some("nt-rosenborg-brann"));
    assert_eq!(rows[0].reference_odds, 1.18);
    assert!(rows[0].source.contains("Book A"));
    assert!(
        rows[0]
            .notes
            .as_deref()
            .is_some_and(|note| note.contains("h2h"))
    );
}

#[test]
fn matches_provider_team_suffixes_and_diacritics() {
    let events = parse_events(
        r#"[
          {
            "id": "fixture-palmeiras-cerro",
            "sport_title": "Copa Libertadores",
            "commence_time": "2026-05-21T00:30:00Z",
            "home_team": "Palmeiras",
            "away_team": "Cerro Porteño",
            "bookmakers": [
              {
                "title": "Book A",
                "markets": [
                  {
                    "key": "h2h",
                    "outcomes": [
                      {"name": "Palmeiras", "price": 1.22},
                      {"name": "Draw", "price": 5.60},
                      {"name": "Cerro Porteño", "price": 9.50}
                    ]
                  }
                ]
              }
            ]
          },
          {
            "id": "fixture-independiente-botafogo",
            "sport_title": "Copa Sudamericana",
            "commence_time": "2026-05-21T00:00:00Z",
            "home_team": "Independiente Petrolero",
            "away_team": "Botafogo",
            "bookmakers": [
              {
                "title": "Book A",
                "markets": [
                  {
                    "key": "h2h",
                    "outcomes": [
                      {"name": "Independiente Petrolero", "price": 7.20},
                      {"name": "Draw", "price": 4.80},
                      {"name": "Botafogo", "price": 1.27}
                    ]
                  }
                ]
              }
            ]
          }
        ]"#,
    )
    .expect("fixture JSON");
    let mut palmeiras = candidate();
    palmeiras.id = "nt-palmeiras-cerro".to_string();
    palmeiras.competition = "Copa Libertadores".to_string();
    palmeiras.event = "Palmeiras - Cerro Porteno".to_string();
    palmeiras.selection = "Cerro Porteno".to_string();
    palmeiras.starts_at = "2026-05-21T02:30:00+02:00".to_string();
    let mut botafogo = candidate();
    botafogo.id = "nt-botafogo-rj".to_string();
    botafogo.competition = "Copa Sudamericana".to_string();
    botafogo.event = "Independiente Petrolero - Botafogo RJ".to_string();
    botafogo.selection = "Botafogo RJ".to_string();
    botafogo.starts_at = "2026-05-21T02:00:00+02:00".to_string();

    let rows = reference_rows_from_events(&[palmeiras, botafogo], &events);

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(
        |row| row.candidate_id.as_deref() == Some("nt-palmeiras-cerro")
            && (row.reference_odds - 9.50).abs() < 0.001
    ));
    assert!(
        rows.iter()
            .any(|row| row.candidate_id.as_deref() == Some("nt-botafogo-rj")
                && (row.reference_odds - 1.27).abs() < 0.001)
    );
}

#[test]
fn maps_fixture_totals_odds_when_market_is_enabled() {
    let events =
        parse_events(include_str!("../../../fixtures/the_odds_api_h2h.json")).expect("fixture");
    let mut candidate = candidate();
    candidate.market = "Over/under".to_string();
    candidate.selection = "Over 2.5 goals".to_string();

    let rows = reference_rows_from_events(&[candidate], &events);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].reference_odds, 1.21);
    assert!(
        rows[0]
            .notes
            .as_deref()
            .is_some_and(|note| note.contains("totals market"))
    );
}

#[test]
fn summarizes_request_and_match_counts_without_secret_values() {
    let events =
        parse_events(include_str!("../../../fixtures/the_odds_api_h2h.json")).expect("fixture");
    let rows = reference_rows_from_events(&[candidate()], &events);
    let stats = FetchStats {
        sport_odds_requests: 5,
        sport_odds_successes: 4,
        event_list_requests: 2,
        event_list_successes: 2,
        event_odds_requests: 1,
        event_odds_successes: 1,
        ..FetchStats::default()
    };

    let summary = provider_summary("The Odds API", &stats, events.len(), &rows, 5);

    assert!(summary.contains("sport odds requests 4/5"));
    assert!(summary.contains("event list requests 2/2"));
    assert!(summary.contains("event odds requests 1/1"));
    assert!(summary.contains("matched 2 reference row(s) for 1 candidate(s)"));
    assert!(summary.contains("bookmaker keys 5/5"));
    assert!(!summary.contains("test-key"));
}

#[test]
fn redacts_api_key_from_provider_errors() {
    let error = sanitize_provider_error(
        "request failed for url https://api.example/v4/sports?sport=x&apiKey=test-key",
        "test-key",
    );

    assert!(error.contains("apiKey=<redacted>"));
    assert!(!error.contains("test-key"));
}

#[test]
fn maps_fixture_double_chance_odds_from_event_odds() {
    let events =
        parse_events(include_str!("../../../fixtures/the_odds_api_h2h.json")).expect("fixture");
    let mut double_chance = candidate();
    double_chance.market = "Double chance".to_string();
    double_chance.selection = "Rosenborg or draw".to_string();

    let rows = reference_rows_from_events(&[double_chance], &events);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].reference_odds, 1.08);
    assert!(
        rows[0]
            .notes
            .as_deref()
            .is_some_and(|note| note.contains("double_chance market"))
    );
}

#[test]
fn finds_event_ids_for_double_chance_candidates() {
    let events =
        parse_events(include_str!("../../../fixtures/the_odds_api_h2h.json")).expect("fixture");
    let mut double_chance = candidate();
    double_chance.market = "Double chance".to_string();
    double_chance.selection = "Rosenborg or draw".to_string();

    let ids = event_ids_matching_market(&[double_chance], &events, "double_chance");

    assert_eq!(ids, vec!["fixture-rosenborg-brann".to_string()]);
}

#[test]
fn supports_norwegian_double_chance_selection_names() {
    let events =
        parse_events(include_str!("../../../fixtures/the_odds_api_h2h.json")).expect("fixture");
    let mut double_chance = candidate();
    double_chance.market = "Dobbeltsjanse".to_string();
    double_chance.selection = "Rosenborg eller uavgjort".to_string();

    let rows = reference_rows_from_events(&[double_chance], &events);

    assert_eq!(rows.len(), 2);
}

#[test]
fn rejects_total_selection_when_point_does_not_match() {
    let events =
        parse_events(include_str!("../../../fixtures/the_odds_api_h2h.json")).expect("fixture");
    let mut candidate = candidate();
    candidate.market = "Over/under".to_string();
    candidate.selection = "Over 1.5 goals".to_string();

    let rows = reference_rows_from_events(&[candidate], &events);

    assert!(rows.is_empty());
}

#[test]
fn compares_offset_and_utc_start_times() {
    let oslo = iso_to_utc_minutes("2026-05-16T18:00:00.000+02:00").expect("oslo time");
    let utc = iso_to_utc_minutes("2026-05-16T16:00:00Z").expect("utc time");

    assert_eq!(oslo, utc);
}
