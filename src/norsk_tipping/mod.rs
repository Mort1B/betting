mod build;
mod client;
mod models;

use crate::domain::{BetCandidate, BettingRules};

use build::{candidates_from_events, compare_candidates};
use client::LiveOddsClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveOddsOptions {
    pub events_per_sport: usize,
    pub earliest_start: Option<String>,
}

impl Default for LiveOddsOptions {
    fn default() -> Self {
        Self {
            events_per_sport: 35,
            earliest_start: None,
        }
    }
}

pub fn load_candidates_from_live_odds(
    rules: &BettingRules,
    options: &LiveOddsOptions,
) -> Result<Vec<BetCandidate>, String> {
    let date = rules
        .date
        .as_deref()
        .ok_or_else(|| "live Norsk Tipping source requires --date YYYY-MM-DD".to_string())?;
    let client = LiveOddsClient::new()?;
    let compact_date = client::compact_date(date)?;
    let sport_types = client.fetch_sport_types(&compact_date)?;
    if sport_types.is_empty() {
        return Err(format!("Norsk Tipping returned no sports for {date}"));
    }

    let mut candidates = Vec::new();
    for sport_type in sport_types {
        let Some(sport_id) = sport_type.id() else {
            continue;
        };
        let events = client.fetch_events(&sport_id, &compact_date, options.events_per_sport)?;
        candidates.extend(candidates_from_events(
            events,
            rules,
            sport_type.display_name(),
            options.earliest_start.as_deref(),
        ));
    }

    candidates.sort_by(compare_candidates);
    if candidates.is_empty() {
        return Err(format!(
            "Norsk Tipping returned no tradable selections between {:.2} and {:.2} for {date}",
            rules.min_odds, rules.max_odds
        ));
    }
    Ok(candidates)
}
