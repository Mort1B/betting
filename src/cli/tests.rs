use super::*;

#[test]
fn parses_csv_source_by_default() {
    let options = CliOptions::parse(
        [
            "examples/norsk_tipping_candidates.csv",
            "--date",
            "2026-05-16",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .expect("valid options");

    match options.source {
        CandidateSource::Csv(path) => assert_eq!(path, "examples/norsk_tipping_candidates.csv"),
        CandidateSource::NorskTippingLive(_) => panic!("expected CSV source"),
    }
    assert_eq!(options.rules.date.as_deref(), Some("2026-05-16"));
}

#[test]
fn parses_norsk_tipping_live_source() {
    let options = CliOptions::parse(
        [
            "--norsk-tipping-live",
            "--date",
            "2026-05-16",
            "--nt-events-per-sport",
            "50",
            "--pick-count",
            "7",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .expect("valid options");

    match options.source {
        CandidateSource::NorskTippingLive(live) => {
            assert_eq!(live.events_per_sport, 50);
            assert!(live.earliest_start.is_none());
        }
        CandidateSource::Csv(_) => panic!("expected live source"),
    }
    assert_eq!(options.rules.pick_count, 7);
}

#[test]
fn parses_norsk_tipping_live_start_cutoff() {
    let options = CliOptions::parse(
        [
            "--norsk-tipping-live",
            "--date",
            "2026-05-16",
            "--nt-earliest-start",
            "2026-05-16T16:00",
            "--nt-latest-start",
            "2026-05-17T05:00",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .expect("valid options");

    match options.source {
        CandidateSource::NorskTippingLive(live) => {
            assert_eq!(live.earliest_start.as_deref(), Some("2026-05-16T16:00"));
            assert_eq!(live.latest_start.as_deref(), Some("2026-05-17T05:00"));
        }
        CandidateSource::Csv(_) => panic!("expected live source"),
    }
    assert_eq!(
        options.rules.latest_start.as_deref(),
        Some("2026-05-17T05:00")
    );
}

#[test]
fn parses_reference_odds_path() {
    let options = CliOptions::parse(
        [
            "--norsk-tipping-live",
            "--date",
            "2026-05-16",
            "--reference-odds",
            "reference_odds.csv",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .expect("valid options");

    assert_eq!(
        options.reference_odds.source_path.as_deref(),
        Some("reference_odds.csv")
    );
}

#[test]
fn parses_the_odds_api_reference_provider() {
    let options = CliOptions::parse(
        [
            "--norsk-tipping-live",
            "--date",
            "2026-05-16",
            "--odds-api-key",
            "test-key",
            "--odds-api-sports",
            "soccer_norway_eliteserien,soccer_sweden_allsvenskan",
            "--odds-api-bookmakers",
            "unibet_se,pinnacle",
            "--odds-api-event-odds-limit",
            "4",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .expect("valid options");

    let provider = options
        .reference_odds
        .providers
        .the_odds_api
        .expect("provider enabled");
    assert_eq!(provider.api_key, "test-key");
    assert_eq!(
        provider.sport_keys,
        vec![
            "soccer_norway_eliteserien".to_string(),
            "soccer_sweden_allsvenskan".to_string()
        ]
    );
    assert_eq!(provider.bookmakers.as_deref(), Some("unibet_se,pinnacle"));
    assert_eq!(provider.event_odds_limit, 4);
}

#[test]
fn defaults_the_odds_api_to_five_bookmakers() {
    let options = CliOptions::parse(
        [
            "--norsk-tipping-live",
            "--date",
            "2026-05-16",
            "--odds-api-key",
            "test-key",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .expect("valid options");

    let provider = options
        .reference_odds
        .providers
        .the_odds_api
        .expect("provider enabled");
    assert_eq!(
        provider.bookmakers.as_deref(),
        Some("unibet_se,pinnacle,betfair_ex_eu,betsson,williamhill")
    );
}

#[test]
fn rejects_more_than_five_odds_api_bookmakers() {
    let error = CliOptions::parse(
        [
            "--norsk-tipping-live",
            "--date",
            "2026-05-16",
            "--odds-api-key",
            "test-key",
            "--odds-api-bookmakers",
            "unibet_se,pinnacle,betfair_ex_eu,betsson,williamhill,nordicbet",
        ]
        .into_iter()
        .map(str::to_string),
    )
    .expect_err("too many bookmakers");

    assert!(error.contains("at most 5"));
}
