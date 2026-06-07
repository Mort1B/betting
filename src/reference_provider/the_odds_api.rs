mod event_match;
mod http;
mod market_match;
mod request_plan;
mod sport_keys;
mod time;

use reqwest::blocking::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::domain::BetCandidate;
use crate::reference::ReferenceOddsRow;

use super::{ReferenceOddsProvider, ReferenceProviderOutput};
use event_match::{event_ids_matching_market, reference_rows_from_events};
use http::send_request;
use request_plan::{FetchStats, RequestedMarkets, clean_sport_keys};

const DEFAULT_BASE_URL: &str = "https://api.the-odds-api.com";
const USER_AGENT: &str = "betting-daily-agent/0.1 by local-user";
const START_TOLERANCE_MINUTES: i64 = 90;
pub const DEFAULT_BOOKMAKERS: &str = "unibet_se,pinnacle,betfair_ex_eu,betsson,williamhill";
pub const MAX_BOOKMAKERS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheOddsApiOptions {
    pub api_key: String,
    pub sport_keys: Vec<String>,
    pub regions: String,
    pub markets: String,
    pub bookmakers: Option<String>,
    pub base_url: String,
    pub commence_time_from: Option<String>,
    pub commence_time_to: Option<String>,
    pub event_odds_limit: usize,
}

impl TheOddsApiOptions {
    pub fn new(api_key: String, sport_keys: Vec<String>) -> Self {
        Self {
            api_key,
            sport_keys: clean_sport_keys(sport_keys),
            regions: "eu".to_string(),
            markets: "h2h".to_string(),
            bookmakers: Some(DEFAULT_BOOKMAKERS.to_string()),
            base_url: DEFAULT_BASE_URL.to_string(),
            commence_time_from: None,
            commence_time_to: None,
            event_odds_limit: 2,
        }
    }
}

pub struct TheOddsApiProvider {
    options: TheOddsApiOptions,
    http: Client,
}

impl TheOddsApiProvider {
    pub fn new(options: TheOddsApiOptions) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(USER_AGENT)
            .build()
            .expect("The Odds API HTTP client should build");
        Self { options, http }
    }

    fn fetch_odds_events_for_sport(
        &self,
        sport_key: &str,
        markets: &str,
    ) -> Result<(Vec<TheOddsApiEvent>, http::CreditUsage), String> {
        let url = format!(
            "{}/v4/sports/{}/odds/",
            self.options.base_url.trim_end_matches('/'),
            sport_key
        );
        let mut request = self.http.get(url).query(&[
            ("apiKey", self.options.api_key.as_str()),
            ("regions", self.options.regions.as_str()),
            ("markets", markets),
            ("oddsFormat", "decimal"),
            ("dateFormat", "iso"),
        ]);

        if let Some(bookmakers) = self.options.bookmakers.as_deref() {
            request = request.query(&[("bookmakers", bookmakers)]);
        }
        if let Some(commence_time_from) = self.options.commence_time_from.as_deref() {
            request = request.query(&[("commenceTimeFrom", commence_time_from)]);
        }
        if let Some(commence_time_to) = self.options.commence_time_to.as_deref() {
            request = request.query(&[("commenceTimeTo", commence_time_to)]);
        }

        let response = send_request(request)?;
        Ok((parse_events(&response.body)?, response.credits))
    }

    fn fetch_event_list_for_sport(
        &self,
        sport_key: &str,
    ) -> Result<(Vec<TheOddsApiEvent>, http::CreditUsage), String> {
        let url = format!(
            "{}/v4/sports/{}/events",
            self.options.base_url.trim_end_matches('/'),
            sport_key
        );
        let mut request = self.http.get(url).query(&[
            ("apiKey", self.options.api_key.as_str()),
            ("dateFormat", "iso"),
        ]);

        if let Some(commence_time_from) = self.options.commence_time_from.as_deref() {
            request = request.query(&[("commenceTimeFrom", commence_time_from)]);
        }
        if let Some(commence_time_to) = self.options.commence_time_to.as_deref() {
            request = request.query(&[("commenceTimeTo", commence_time_to)]);
        }

        let response = send_request(request)?;
        Ok((parse_events(&response.body)?, response.credits))
    }

    fn fetch_event_odds(
        &self,
        sport_key: &str,
        event_id: &str,
        market: &str,
    ) -> Result<(TheOddsApiEvent, http::CreditUsage), String> {
        let url = format!(
            "{}/v4/sports/{}/events/{}/odds",
            self.options.base_url.trim_end_matches('/'),
            sport_key,
            event_id
        );
        let mut request = self.http.get(url).query(&[
            ("apiKey", self.options.api_key.as_str()),
            ("regions", self.options.regions.as_str()),
            ("markets", market),
            ("oddsFormat", "decimal"),
            ("dateFormat", "iso"),
        ]);

        if let Some(bookmakers) = self.options.bookmakers.as_deref() {
            request = request.query(&[("bookmakers", bookmakers)]);
        }

        let response = send_request(request)?;
        Ok((parse_event(&response.body)?, response.credits))
    }
}

impl ReferenceOddsProvider for TheOddsApiProvider {
    fn name(&self) -> &'static str {
        "The Odds API"
    }

    fn fetch_rows(&self, candidates: &[BetCandidate]) -> ReferenceProviderOutput {
        let mut output = ReferenceProviderOutput::default();
        let mut events = Vec::new();
        let mut stats = FetchStats::default();
        let requested_markets = RequestedMarkets::parse(&self.options.markets);

        let sport_keys = sport_keys::resolve_sport_keys(&self.options.sport_keys, candidates);
        for sport_key in &sport_keys {
            if let Some(featured_markets) = requested_markets.featured_query() {
                stats.sport_odds_requests += 1;
                match self.fetch_odds_events_for_sport(sport_key, &featured_markets) {
                    Ok((mut sport_events, credits)) => {
                        stats.sport_odds_successes += 1;
                        stats.record_credits(&credits);
                        events.append(&mut sport_events);
                    }
                    Err(error) => push_sanitized_error(
                        &mut output,
                        self.name(),
                        sport_key,
                        &self.options.api_key,
                        &error,
                    ),
                }
            }

            if requested_markets.needs_event_odds() {
                fetch_event_market_rows(
                    self,
                    candidates,
                    sport_key,
                    &requested_markets,
                    &mut events,
                    &mut stats,
                    &mut output,
                );
            }
        }

        output.rows = reference_rows_from_events(candidates, &events);
        output.summaries.push(provider_summary(
            self.name(),
            &stats,
            events.len(),
            &output.rows,
            bookmaker_count(self.options.bookmakers.as_deref()),
        ));
        if !events.is_empty() && output.rows.is_empty() {
            output.notes.push(format!(
                "reference odds provider {} returned events but no prices matched Norsk Tipping candidates",
                self.name()
            ));
        } else if events.is_empty() && output.notes.is_empty() {
            output.notes.push(format!(
                "reference odds provider {} returned no events",
                self.name()
            ));
        }
        output
    }
}

fn fetch_event_market_rows(
    provider: &TheOddsApiProvider,
    candidates: &[BetCandidate],
    sport_key: &str,
    requested_markets: &RequestedMarkets,
    events: &mut Vec<TheOddsApiEvent>,
    stats: &mut FetchStats,
    output: &mut ReferenceProviderOutput,
) {
    stats.event_list_requests += 1;
    let event_list = match provider.fetch_event_list_for_sport(sport_key) {
        Ok((event_list, credits)) => {
            stats.event_list_successes += 1;
            stats.record_credits(&credits);
            event_list
        }
        Err(error) => {
            push_sanitized_error(
                output,
                provider.name(),
                sport_key,
                &provider.options.api_key,
                &error,
            );
            return;
        }
    };

    for event_market in &requested_markets.event_markets {
        let matching_ids = event_ids_matching_market(candidates, &event_list, event_market);
        for event_id in matching_ids {
            if stats.event_odds_requests >= provider.options.event_odds_limit {
                output.notes.push(format!(
                    "reference odds provider {} skipped additional {event_market} event odds after limit {}",
                    provider.name(),
                    provider.options.event_odds_limit
                ));
                return;
            }

            stats.event_odds_requests += 1;
            match provider.fetch_event_odds(sport_key, &event_id, event_market) {
                Ok((event, credits)) => {
                    stats.event_odds_successes += 1;
                    stats.record_credits(&credits);
                    events.push(event);
                }
                Err(error) => push_sanitized_error(
                    output,
                    provider.name(),
                    sport_key,
                    &provider.options.api_key,
                    &error,
                ),
            }
        }
    }
}

fn push_sanitized_error(
    output: &mut ReferenceProviderOutput,
    provider: &str,
    sport_key: &str,
    api_key: &str,
    error: &str,
) {
    let error = sanitize_provider_error(error, api_key);
    output.notes.push(format!(
        "reference odds provider {provider} {sport_key}: {error}"
    ));
}

fn provider_summary(
    provider: &str,
    stats: &FetchStats,
    event_count: usize,
    rows: &[ReferenceOddsRow],
    bookmaker_count: usize,
) -> String {
    let mut summary = format!(
        "{provider}: sport odds requests {}/{}, event list requests {}/{}, event odds requests {}/{}, returned {event_count} event(s), matched {} reference row(s) for {} candidate(s), bookmaker keys {bookmaker_count}/{MAX_BOOKMAKERS}",
        stats.sport_odds_successes,
        stats.sport_odds_requests,
        stats.event_list_successes,
        stats.event_list_requests,
        stats.event_odds_successes,
        stats.event_odds_requests,
        rows.len(),
        matched_candidate_count(rows)
    );
    if let Some(credits) = stats.credit_summary() {
        summary.push_str(&format!(", API credits {credits}"));
    }
    summary
}

fn matched_candidate_count(rows: &[ReferenceOddsRow]) -> usize {
    let mut candidate_ids = rows
        .iter()
        .filter_map(|row| row.candidate_id.as_deref())
        .collect::<Vec<_>>();
    candidate_ids.sort_unstable();
    candidate_ids.dedup();
    candidate_ids.len()
}

fn bookmaker_count(bookmakers: Option<&str>) -> usize {
    bookmakers
        .map(|bookmakers| {
            bookmakers
                .split(',')
                .map(str::trim)
                .filter(|bookmaker| !bookmaker.is_empty())
                .count()
        })
        .unwrap_or(0)
}

fn sanitize_provider_error(error: &str, api_key: &str) -> String {
    if api_key.trim().is_empty() {
        return error.to_string();
    }
    error.replace(api_key, "<redacted>")
}

#[derive(Debug, Clone, Deserialize)]
struct TheOddsApiEvent {
    id: Option<String>,
    sport_title: Option<String>,
    commence_time: String,
    home_team: String,
    away_team: String,
    #[serde(default)]
    bookmakers: Vec<TheOddsApiBookmaker>,
}

#[derive(Debug, Clone, Deserialize)]
struct TheOddsApiBookmaker {
    title: String,
    last_update: Option<String>,
    #[serde(default)]
    markets: Vec<TheOddsApiMarket>,
}

#[derive(Debug, Clone, Deserialize)]
struct TheOddsApiMarket {
    key: String,
    last_update: Option<String>,
    #[serde(default)]
    outcomes: Vec<TheOddsApiOutcome>,
}

#[derive(Debug, Clone, Deserialize)]
struct TheOddsApiOutcome {
    name: String,
    price: f64,
    point: Option<f64>,
}

fn parse_events(body: &str) -> Result<Vec<TheOddsApiEvent>, String> {
    serde_json::from_str(body).map_err(|error| format!("invalid JSON: {error}"))
}

fn parse_event(body: &str) -> Result<TheOddsApiEvent, String> {
    serde_json::from_str(body).map_err(|error| format!("invalid JSON: {error}"))
}

fn round_to_two_decimals(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests;
