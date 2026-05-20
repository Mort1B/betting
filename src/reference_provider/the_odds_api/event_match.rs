use crate::domain::BetCandidate;
use crate::reference::ReferenceOddsRow;

use super::market_match::{candidate_matches_market_outcome, candidate_supports_market_key};
use super::time::iso_to_utc_minutes;
use super::{
    START_TOLERANCE_MINUTES, TheOddsApiBookmaker, TheOddsApiEvent, TheOddsApiMarket,
    round_to_two_decimals,
};

const STALE_UPDATE_WARNING_MINUTES: i64 = 12 * 60;

pub(super) fn reference_rows_from_events(
    candidates: &[BetCandidate],
    events: &[TheOddsApiEvent],
) -> Vec<ReferenceOddsRow> {
    let mut rows = Vec::new();
    for candidate in candidates {
        let Some(candidate_teams) = candidate_event_teams(&candidate.event) else {
            continue;
        };
        let Some(candidate_start) = iso_to_utc_minutes(&candidate.starts_at) else {
            continue;
        };

        for event in events {
            if !candidate_matches_event(&candidate_teams, candidate_start, event) {
                continue;
            }
            push_event_rows(&mut rows, candidate, event);
        }
    }
    rows
}

pub(super) fn event_ids_matching_market(
    candidates: &[BetCandidate],
    events: &[TheOddsApiEvent],
    market_key: &str,
) -> Vec<String> {
    let mut event_ids = Vec::new();
    for candidate in candidates {
        if !candidate_supports_market_key(candidate, market_key) {
            continue;
        }
        let Some(candidate_teams) = candidate_event_teams(&candidate.event) else {
            continue;
        };
        let Some(candidate_start) = iso_to_utc_minutes(&candidate.starts_at) else {
            continue;
        };

        for event in events {
            if !candidate_matches_event(&candidate_teams, candidate_start, event) {
                continue;
            }
            if let Some(event_id) = event.id.as_deref() {
                push_unique(&mut event_ids, event_id);
            }
        }
    }
    event_ids
}

fn push_event_rows(
    rows: &mut Vec<ReferenceOddsRow>,
    candidate: &BetCandidate,
    event: &TheOddsApiEvent,
) {
    for bookmaker in &event.bookmakers {
        for market in &bookmaker.markets {
            for outcome in market.outcomes.iter().filter(|outcome| outcome.price > 1.0) {
                if !candidate_matches_market_outcome(candidate, market, outcome) {
                    continue;
                }
                rows.push(ReferenceOddsRow {
                    candidate_id: Some(candidate.id.clone()),
                    sport: Some(candidate.sport.clone()),
                    competition: Some(candidate.competition.clone()),
                    event: Some(candidate.event.clone()),
                    market: Some(candidate.market.clone()),
                    selection: Some(candidate.selection.clone()),
                    reference_odds: round_to_two_decimals(outcome.price),
                    source: format!("The Odds API {}", bookmaker.title),
                    notes: Some(provider_note(event, bookmaker, market)),
                });
            }
        }
    }
}

fn provider_note(
    event: &TheOddsApiEvent,
    bookmaker: &TheOddsApiBookmaker,
    market: &TheOddsApiMarket,
) -> String {
    let sport = event.sport_title.as_deref().unwrap_or("soccer");
    let update = bookmaker
        .last_update
        .as_deref()
        .or(market.last_update.as_deref())
        .unwrap_or("unknown update");
    let freshness = update_freshness_note(event, update);
    format!(
        "{sport} {} market matched by teams and start time; API start {}; last update {update}; {freshness}",
        market.key, event.commence_time
    )
}

fn update_freshness_note(event: &TheOddsApiEvent, update: &str) -> String {
    let Some(event_start) = iso_to_utc_minutes(&event.commence_time) else {
        return "source freshness unknown".to_string();
    };
    let Some(update_time) = iso_to_utc_minutes(update) else {
        return "source freshness unknown".to_string();
    };
    let age_before_start = event_start - update_time;
    if age_before_start > STALE_UPDATE_WARNING_MINUTES {
        format!(
            "source freshness warning: odds update {:.1}h before kickoff",
            age_before_start as f64 / 60.0
        )
    } else {
        "source freshness acceptable".to_string()
    }
}

fn candidate_matches_event(
    candidate_teams: &(String, String),
    candidate_start: i64,
    event: &TheOddsApiEvent,
) -> bool {
    if !teams_match(candidate_teams, event) {
        return false;
    }
    let Some(event_start) = iso_to_utc_minutes(&event.commence_time) else {
        return false;
    };
    (candidate_start - event_start).abs() <= START_TOLERANCE_MINUTES
}

fn teams_match(candidate_teams: &(String, String), event: &TheOddsApiEvent) -> bool {
    let home = normalize_key(&event.home_team);
    let away = normalize_key(&event.away_team);
    (candidate_teams.0 == home && candidate_teams.1 == away)
        || (candidate_teams.0 == away && candidate_teams.1 == home)
}

fn candidate_event_teams(event: &str) -> Option<(String, String)> {
    for separator in [" - ", " vs. ", " vs ", " v ", " @ "] {
        if let Some((left, right)) = event.split_once(separator) {
            let left = normalize_key(left);
            let right = normalize_key(right);
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right));
            }
        }
    }
    None
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}
