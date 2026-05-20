use crate::domain::BetCandidate;

use super::{TheOddsApiMarket, TheOddsApiOutcome};

pub(super) fn candidate_matches_market_outcome(
    candidate: &BetCandidate,
    market: &TheOddsApiMarket,
    outcome: &TheOddsApiOutcome,
) -> bool {
    match market.key.as_str() {
        "h2h" => {
            is_h2h_candidate_market(&candidate.market)
                && outcome_matches_selection(&candidate.selection, &outcome.name)
        }
        "totals" => totals_selection(&candidate.market, &candidate.selection)
            .is_some_and(|selection| outcome_matches_total(&selection, outcome)),
        "double_chance" => {
            candidate_supports_market_key(candidate, &market.key)
                && double_chance_legs(&candidate.selection)
                    .is_some_and(|selection| outcome_matches_double_chance(&selection, outcome))
        }
        _ => false,
    }
}

pub(super) fn candidate_supports_market_key(candidate: &BetCandidate, market_key: &str) -> bool {
    match market_key {
        "double_chance" => is_double_chance_candidate(&candidate.market, &candidate.selection),
        _ => false,
    }
}

fn is_h2h_candidate_market(market: &str) -> bool {
    matches!(
        normalize_key(market).as_str(),
        "main market"
            | "match winner"
            | "match result"
            | "full time result"
            | "fulltime result"
            | "moneyline"
            | "winner"
            | "1x2"
    )
}

fn outcome_matches_selection(selection: &str, outcome_name: &str) -> bool {
    normalize_key(selection) == normalize_key(outcome_name)
}

fn is_double_chance_candidate(market: &str, selection: &str) -> bool {
    let market = normalize_key(market);
    let selection = normalize_key(selection);
    market.contains("double chance")
        || market.contains("dobbeltsjanse")
        || market.contains("dobbelsjanse")
        || selection.contains(" or draw")
        || selection.contains(" draw or")
        || selection.contains(" eller uavgjort")
        || selection.contains(" uavgjort eller")
}

fn outcome_matches_double_chance(
    selection: &DoubleChanceSelection,
    outcome: &TheOddsApiOutcome,
) -> bool {
    double_chance_legs(&outcome.name).is_some_and(|outcome_legs| selection == &outcome_legs)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoubleChanceSelection {
    legs: Vec<String>,
}

fn double_chance_legs(value: &str) -> Option<DoubleChanceSelection> {
    let canonical = canonical_double_chance_text(value);
    let separator = if canonical.contains(" or ") {
        " or "
    } else if canonical.contains(" / ") {
        " / "
    } else {
        return None;
    };
    let mut legs = canonical
        .split(separator)
        .map(str::trim)
        .filter(|leg| !leg.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if legs.len() != 2 {
        return None;
    }
    legs.sort_unstable();
    legs.dedup();
    if legs.len() == 2 {
        Some(DoubleChanceSelection { legs })
    } else {
        None
    }
}

fn canonical_double_chance_text(value: &str) -> String {
    let normalized = value
        .replace('/', " / ")
        .replace('-', " - ")
        .chars()
        .flat_map(char::to_lowercase)
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '/' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();
    normalized
        .split_whitespace()
        .map(|word| match word {
            "eller" => "or",
            "uavgjort" => "draw",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TotalsSelection {
    side: TotalsSide,
    point: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TotalsSide {
    Over,
    Under,
}

fn totals_selection(market: &str, selection: &str) -> Option<TotalsSelection> {
    let normalized_market = normalize_key(market);
    let normalized_selection = normalize_key(selection);
    let is_totals_market = normalized_market.contains("over under")
        || normalized_market.contains("total")
        || normalized_market.contains("mål")
        || normalized_selection.starts_with("over ")
        || normalized_selection.starts_with("under ");
    if !is_totals_market {
        return None;
    }

    let side = if normalized_selection.starts_with("over ") {
        TotalsSide::Over
    } else if normalized_selection.starts_with("under ") {
        TotalsSide::Under
    } else {
        return None;
    };
    Some(TotalsSelection {
        side,
        point: extract_decimal(selection)?,
    })
}

fn outcome_matches_total(selection: &TotalsSelection, outcome: &TheOddsApiOutcome) -> bool {
    let outcome_side = match normalize_key(&outcome.name).as_str() {
        "over" => TotalsSide::Over,
        "under" => TotalsSide::Under,
        _ => return false,
    };
    let Some(point) = outcome.point else {
        return false;
    };
    selection.side == outcome_side && (selection.point - point).abs() < 0.001
}

fn extract_decimal(value: &str) -> Option<f64> {
    let mut current = String::new();
    for ch in value.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            current.push(if ch == ',' { '.' } else { ch });
        } else if !current.is_empty() {
            break;
        }
    }
    current.parse().ok()
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
