use super::*;

fn candidate() -> BetCandidate {
    BetCandidate {
        id: "nt-123".to_string(),
        sport: "Football".to_string(),
        competition: "Eliteserien".to_string(),
        event: "Rosenborg - Brann".to_string(),
        market: "Main market".to_string(),
        selection: "Rosenborg".to_string(),
        norsk_tipping_odds: 1.22,
        model_probability: None,
        reference_odds: None,
        confidence: Some(0.72),
        starts_at: "2026-05-16T18:00:00+02:00".to_string(),
        notes: "live import".to_string(),
    }
}

#[test]
fn enriches_by_candidate_id_with_consensus_reference_odds() {
    let rows = parse_reference_rows(
        "candidate_id,source,reference_odds,notes\n\
         nt-123,Book A,1.15,market close\n\
         nt-123,Book B,1.17,market close",
    )
    .expect("valid rows");

    let enriched = enrich_candidates(vec![candidate()], &rows);
    assert_eq!(enriched[0].reference_odds, Some(1.16));
    assert!(enriched[0].notes.contains("reference odds consensus 1.16"));
}

#[test]
fn enriches_by_event_market_and_selection() {
    let rows = parse_reference_rows(
        "event,market,selection,reference_odds,source\n\
         \"rosenborg brann\",\"main-market\",Rosenborg,1.16,Book A",
    )
    .expect("valid rows");

    let enriched = enrich_candidates(vec![candidate()], &rows);
    assert_eq!(enriched[0].reference_odds, Some(1.16));
}

#[test]
fn tuple_match_preserves_optional_sport_and_competition_constraints() {
    let rows = parse_reference_rows(
        "event,market,selection,sport,competition,reference_odds,source\n\
         Rosenborg - Brann,Main market,Rosenborg,Tennis,Eliteserien,1.10,Bad sport\n\
         Rosenborg - Brann,Main market,Rosenborg,Football,Obosligaen,1.11,Bad comp\n\
         Rosenborg - Brann,Main market,Rosenborg,Football,Eliteserien,1.18,Book A",
    )
    .expect("valid rows");

    let enriched = enrich_candidates(vec![candidate()], &rows);
    assert_eq!(enriched[0].reference_odds, Some(1.18));
    assert!(enriched[0].notes.contains("Book A 1.18"));
    assert!(!enriched[0].notes.contains("Bad sport"));
    assert!(!enriched[0].notes.contains("Bad comp"));
}

#[test]
fn combines_id_and_tuple_matches_in_source_order() {
    let rows = parse_reference_rows(
        "candidate_id,event,market,selection,source,reference_odds\n\
         ,Rosenborg - Brann,Main market,Rosenborg,Tuple A,1.18\n\
         nt-123,,,,Id B,1.16\n\
         ,Rosenborg - Brann,Main market,Rosenborg,Tuple C,1.14",
    )
    .expect("valid rows");

    let enriched = enrich_candidates(vec![candidate()], &rows);
    assert_eq!(enriched[0].reference_odds, Some(1.16));
    assert!(
        enriched[0]
            .notes
            .contains("Tuple A 1.18; Id B 1.16; Tuple C 1.14")
    );
}

#[test]
fn leaves_candidate_without_reference_on_no_match() {
    let rows = parse_reference_rows(
        "event,market,selection,reference_odds,source\n\
         Brann - Viking,Main market,Brann,1.20,Book A",
    )
    .expect("valid rows");

    let enriched = enrich_candidates(vec![candidate()], &rows);
    assert_eq!(enriched[0].reference_odds, None);
    assert_eq!(enriched[0].notes, "live import");
}

#[test]
fn keeps_existing_reference_odds() {
    let rows =
        parse_reference_rows("candidate_id,reference_odds\nnt-123,1.15").expect("valid rows");
    let mut candidate = candidate();
    candidate.reference_odds = Some(1.20);

    let enriched = enrich_candidates(vec![candidate], &rows);
    assert_eq!(enriched[0].reference_odds, Some(1.20));
}

#[test]
fn rejects_rows_without_match_key() {
    let error =
        parse_reference_rows("source,reference_odds\nBook A,1.15").expect_err("invalid row");
    assert!(error.contains("candidate_id"));
}
