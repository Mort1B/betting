use std::cmp::Ordering;
use std::collections::HashSet;

use crate::domain::{BetCandidate, BettingRules};

use super::models::Event;

pub(crate) fn candidates_from_events(
    events: Vec<Event>,
    rules: &BettingRules,
    sport_fallback: String,
    earliest_start: Option<&str>,
) -> Vec<BetCandidate> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for event in events {
        let Some(event_id) = event.idfoevent.as_deref() else {
            continue;
        };
        if event_id == "_TOKEN_" {
            continue;
        }
        let Some(starts_at) = event.tsstart.as_deref() else {
            continue;
        };
        if starts_before_cutoff(starts_at, earliest_start) {
            continue;
        }
        let event_name = event.event_name();
        let competition = event
            .tournamentname
            .clone()
            .unwrap_or_else(|| "Norsk Tipping Oddsen".to_string());
        let sport = event
            .sporttypename
            .clone()
            .unwrap_or_else(|| sport_fallback.clone());

        for market in event
            .markets
            .iter()
            .filter(|market| market.is_candidate_market())
        {
            let market_name = market.display_name();
            let market_id = market.identifier();

            for selection in market.selections().filter(|selection| selection.is_live()) {
                let Some(selection_name) = selection.name() else {
                    continue;
                };
                let Some(decimal_odds) = selection.decimal_odds() else {
                    continue;
                };
                if decimal_odds < rules.min_odds || decimal_odds > rules.max_odds {
                    continue;
                }

                let selection_id = selection.identifier(selection_name);
                let candidate_id = format!(
                    "nt-{}-{}-{}",
                    safe_id_part(event_id),
                    safe_id_part(&market_id),
                    safe_id_part(&selection_id)
                );
                if !seen.insert(candidate_id.clone()) {
                    continue;
                }

                candidates.push(BetCandidate {
                    id: candidate_id,
                    sport: sport.clone(),
                    competition: competition.clone(),
                    event: event_name.clone(),
                    market: market_name.clone(),
                    selection: selection_name.to_string(),
                    norsk_tipping_odds: decimal_odds,
                    model_probability: None,
                    reference_odds: None,
                    confidence: Some(live_confidence(decimal_odds)),
                    starts_at: starts_at.to_string(),
                    notes: format!(
                        "live Norsk Tipping import; probability starts from market-implied price and context checks; event_id={event_id}; market={market_name}; selection_id={selection_id}"
                    ),
                });
            }
        }
    }

    candidates
}

pub(crate) fn compare_candidates(a: &BetCandidate, b: &BetCandidate) -> Ordering {
    b.implied_probability()
        .partial_cmp(&a.implied_probability())
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.starts_at.cmp(&b.starts_at))
        .then_with(|| a.event.cmp(&b.event))
}

fn live_confidence(decimal_odds: f64) -> f64 {
    (1.0 / decimal_odds - 0.05).clamp(0.72, 0.85)
}

fn safe_id_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn starts_before_cutoff(starts_at: &str, earliest_start: Option<&str>) -> bool {
    let Some(earliest_start) = earliest_start else {
        return false;
    };
    starts_at
        .get(..earliest_start.len())
        .is_some_and(|prefix| prefix < earliest_start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::norsk_tipping::models::ContentResponse;

    #[test]
    fn builds_candidates_from_head_markets_inside_target_band() {
        let response: ContentResponse<Event> = serde_json::from_str(
            r#"{
              "data": [{
                "idfoevent": "8262698.1",
                "participantname_home": "Academico",
                "participantname_away": "Sporting B",
                "sporttypename": "Fotball",
                "tournamentname": "Portugal - Segunda Liga",
                "tsstart": "2026-05-16T12:00:00.000+02:00",
                "markets": [{
                  "idfomarket": "m1",
                  "name": "HUB",
                  "isheadmarket": true,
                  "istradable": true,
                  "selections": [{
                    "idfoselection": "s1",
                    "name": "Academico",
                    "currentpriceup": "3",
                    "currentpricedown": "20",
                    "idfobolifestate": "N"
                  }, {
                    "idfoselection": "s2",
                    "name": "Draw",
                    "currentpriceup": "3",
                    "currentpricedown": "1",
                    "idfobolifestate": "N"
                  }]
                }]
              }]
            }"#,
        )
        .expect("valid fixture");

        let candidates = candidates_from_events(
            response.data,
            &BettingRules::default(),
            "Fotball".to_string(),
            None,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].event, "Academico - Sporting B");
        assert_eq!(candidates[0].market, "Main market");
        assert_eq!(candidates[0].selection, "Academico");
        assert_eq!(candidates[0].norsk_tipping_odds, 1.15);
        assert!(candidates[0].model_probability.is_none());
        assert!(candidates[0].reference_odds.is_none());
    }

    #[test]
    fn skips_events_before_live_cutoff() {
        let response: ContentResponse<Event> = serde_json::from_str(
            r#"{
              "data": [{
                "idfoevent": "8262698.1",
                "name": "Started event",
                "sporttypename": "Fotball",
                "tournamentname": "Cup",
                "tsstart": "2026-05-16T12:00:00.000+02:00",
                "markets": [{
                  "name": "HUB",
                  "isheadmarket": true,
                  "istradable": true,
                  "selections": [{
                    "idfoselection": "s1",
                    "name": "Home",
                    "currentpriceup": "3",
                    "currentpricedown": "20",
                    "idfobolifestate": "N"
                  }]
                }]
              }]
            }"#,
        )
        .expect("valid fixture");

        let candidates = candidates_from_events(
            response.data,
            &BettingRules::default(),
            "Fotball".to_string(),
            Some("2026-05-16T16:00"),
        );

        assert!(candidates.is_empty());
    }
}
