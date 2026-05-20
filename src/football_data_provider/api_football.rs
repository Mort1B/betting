mod context;
mod fixtures;
mod models;
mod stats;
#[cfg(test)]
mod tests;
mod time;

use std::collections::HashMap;
use std::time::Duration;

use crate::domain::{BetCandidate, BettingRules};
use crate::football_data_provider::{FootballContextProvider, FootballDataResult};

use context::{
    append_availability_coverage_note, append_fixture_note, append_form_notes, append_injury_notes,
    append_standings_coverage_note, append_standings_notes, match_candidates,
};
use models::{
    ApiFixture, ApiFootballEnvelope, ApiInjury, ApiLeagueCoverage, ApiLeagueCoverageResponse,
    ApiStandingResponse, ApiStandingRow,
};
use stats::ProviderStats;

const DEFAULT_BASE_URL: &str = "https://v3.football.api-sports.io";
const DEFAULT_TIMEZONE: &str = "Europe/Oslo";

#[derive(Debug, Clone, PartialEq)]
pub struct ApiFootballOptions {
    pub api_key: String,
    pub base_url: String,
    pub timezone: String,
    pub max_context_fixtures: usize,
    pub max_form_teams: usize,
}

impl ApiFootballOptions {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
            timezone: DEFAULT_TIMEZONE.to_string(),
            max_context_fixtures: 2,
            max_form_teams: 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiFootballProvider {
    options: ApiFootballOptions,
}

impl ApiFootballProvider {
    pub fn new(options: ApiFootballOptions) -> Self {
        Self { options }
    }
}

impl FootballContextProvider for ApiFootballProvider {
    fn enrich_candidates(
        &self,
        candidates: Vec<BetCandidate>,
        rules: &BettingRules,
    ) -> FootballDataResult {
        let Some(date) = rules.date.as_deref() else {
            return FootballDataResult {
                candidates,
                provider_report_notes: vec![
                    "API-Football skipped: report date is required".to_string(),
                ],
            };
        };

        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                return FootballDataResult {
                    candidates,
                    provider_report_notes: vec![format!(
                        "API-Football skipped: failed to build HTTP client: {error}"
                    )],
                };
            }
        };

        let mut stats = ProviderStats::default();
        let mut notes = Vec::new();
        let fixture_dates = fixtures::fixture_dates_for_candidates(&candidates, date);
        let fixtures =
            match self.fetch_fixtures_for_dates(&client, &fixture_dates, &mut stats, &mut notes) {
                Ok(fixtures) => fixtures,
                Err(error) => {
                    let candidate_count = candidates.len();
                    return FootballDataResult {
                        candidates,
                        provider_report_notes: vec![
                            stats.summary(0, candidate_count),
                            format!("API-Football fixture request failed: {error}"),
                        ],
                    };
                }
            };

        let mut candidates = candidates;
        let matches = match_candidates(&candidates, &fixtures);
        let mut injury_cache = HashMap::new();
        let mut coverage_cache = HashMap::new();
        let mut standings_cache = HashMap::new();
        let mut team_form_cache = HashMap::new();

        for candidate_match in matches.iter().take(self.options.max_context_fixtures) {
            let fixture = &fixtures[candidate_match.fixture_index];
            let candidate = &mut candidates[candidate_match.candidate_index];
            append_fixture_note(candidate, fixture);
            let league_key = league_season_key(fixture);
            let coverage = match league_key {
                Some(key) => coverage_cache
                    .entry(key)
                    .or_insert_with(
                        || match self.fetch_league_coverage(&client, key, &mut stats) {
                            Ok(coverage) => coverage,
                            Err(error) => {
                                notes.push(format!(
                                    "API-Football coverage request failed for {} {}: {error}",
                                    fixture.league.name, key.season
                                ));
                                None
                            }
                        },
                    )
                    .clone(),
                None => {
                    notes.push(format!(
                        "API-Football coverage skipped for fixture {}: missing league id or season",
                        fixture.fixture.id
                    ));
                    None
                }
            };

            match coverage.as_ref().and_then(|coverage| coverage.injuries) {
                Some(true) => {
                    let injuries = injury_cache.entry(fixture.fixture.id).or_insert_with(|| {
                        match self.fetch_injuries(&client, fixture.fixture.id, &mut stats) {
                            Ok(injuries) => Some(injuries),
                            Err(error) => {
                                notes.push(format!(
                                    "API-Football injury request failed for fixture {}: {error}",
                                    fixture.fixture.id
                                ));
                                None
                            }
                        }
                    });
                    if let Some(injuries) = injuries.as_ref() {
                        append_injury_notes(candidate, fixture, injuries);
                    }
                }
                Some(false) => append_availability_coverage_note(candidate, fixture, "unavailable"),
                None => append_availability_coverage_note(candidate, fixture, "not confirmed"),
            }

            match coverage.as_ref().and_then(|coverage| coverage.standings) {
                Some(true) => {
                    if let Some(key) = league_key {
                        let standings = standings_cache.entry(key).or_insert_with(|| {
                            match self.fetch_standings(&client, key, &mut stats) {
                                Ok(standings) => standings,
                                Err(error) => {
                                    notes.push(format!(
                                        "API-Football standings request failed for {} {}: {error}",
                                        fixture.league.name, key.season
                                    ));
                                    Vec::new()
                                }
                            }
                        });
                        append_standings_notes(candidate, fixture, standings);
                    }
                }
                Some(false) => append_standings_coverage_note(candidate, fixture, "unavailable"),
                None => append_standings_coverage_note(candidate, fixture, "not confirmed"),
            }

            for team in [&fixture.teams.home, &fixture.teams.away] {
                if team_form_cache.contains_key(&team.id) {
                    continue;
                }
                if team_form_cache.len() >= self.options.max_form_teams {
                    continue;
                }
                match self.fetch_team_form(&client, team.id, &mut stats) {
                    Ok(form_fixtures) => {
                        team_form_cache.insert(team.id, form_fixtures);
                    }
                    Err(error) => notes.push(format!(
                        "API-Football form request failed for {}: {error}",
                        team.name
                    )),
                }
            }

            append_form_notes(
                &mut candidates[candidate_match.candidate_index],
                fixture,
                &team_form_cache,
            );
        }

        let mut provider_report_notes = vec![stats.summary(matches.len(), candidates.len())];
        provider_report_notes.extend(notes.into_iter().take(3));

        FootballDataResult {
            candidates,
            provider_report_notes,
        }
    }
}

impl ApiFootballProvider {
    fn fetch_fixtures_by_date(
        &self,
        client: &reqwest::blocking::Client,
        date: &str,
        stats: &mut ProviderStats,
    ) -> Result<Vec<ApiFixture>, String> {
        stats.fixture_requests += 1;
        let body = self
            .request(
                client,
                "fixtures",
                &[("date", date), ("timezone", &self.options.timezone)],
            )
            .map_err(|error| self.public_error(&error))?;
        stats.fixture_success += 1;
        parse_envelope::<ApiFixture>(&body)
    }

    fn fetch_injuries(
        &self,
        client: &reqwest::blocking::Client,
        fixture_id: u64,
        stats: &mut ProviderStats,
    ) -> Result<Vec<ApiInjury>, String> {
        stats.injury_requests += 1;
        let fixture_id = fixture_id.to_string();
        let body = self
            .request(client, "injuries", &[("fixture", &fixture_id)])
            .map_err(|error| self.public_error(&error))?;
        stats.injury_success += 1;
        parse_envelope::<ApiInjury>(&body)
    }

    fn fetch_league_coverage(
        &self,
        client: &reqwest::blocking::Client,
        key: LeagueSeasonKey,
        stats: &mut ProviderStats,
    ) -> Result<Option<ApiLeagueCoverage>, String> {
        stats.coverage_requests += 1;
        let league_id = key.league_id.to_string();
        let season = key.season.to_string();
        let body = self
            .request(
                client,
                "leagues",
                &[("id", &league_id), ("season", &season)],
            )
            .map_err(|error| self.public_error(&error))?;
        stats.coverage_success += 1;
        let leagues = parse_envelope::<ApiLeagueCoverageResponse>(&body)?;
        Ok(leagues
            .into_iter()
            .find(|league| league.league.id == key.league_id)
            .and_then(|league| {
                league
                    .seasons
                    .into_iter()
                    .find(|season| season.year == key.season)
                    .map(|season| season.coverage)
            }))
    }

    fn fetch_standings(
        &self,
        client: &reqwest::blocking::Client,
        key: LeagueSeasonKey,
        stats: &mut ProviderStats,
    ) -> Result<Vec<ApiStandingRow>, String> {
        stats.standings_requests += 1;
        let league_id = key.league_id.to_string();
        let season = key.season.to_string();
        let body = self
            .request(
                client,
                "standings",
                &[("league", &league_id), ("season", &season)],
            )
            .map_err(|error| self.public_error(&error))?;
        stats.standings_success += 1;
        let standings = parse_envelope::<ApiStandingResponse>(&body)?;
        Ok(flatten_standings(standings))
    }

    fn fetch_team_form(
        &self,
        client: &reqwest::blocking::Client,
        team_id: u64,
        stats: &mut ProviderStats,
    ) -> Result<Vec<ApiFixture>, String> {
        stats.form_requests += 1;
        let team_id = team_id.to_string();
        let body = self
            .request(
                client,
                "fixtures",
                &[
                    ("team", &team_id),
                    ("last", "5"),
                    ("timezone", &self.options.timezone),
                ],
            )
            .map_err(|error| self.public_error(&error))?;
        stats.form_success += 1;
        parse_envelope::<ApiFixture>(&body)
    }

    fn request(
        &self,
        client: &reqwest::blocking::Client,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<String, String> {
        let url = format!("{}/{}", self.options.base_url.trim_end_matches('/'), path);
        let response = client
            .get(url)
            .header("x-apisports-key", &self.options.api_key)
            .query(query)
            .send()
            .map_err(|error| format!("request failed: {error}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("HTTP {status}"));
        }
        response
            .text()
            .map_err(|error| format!("body read failed: {error}"))
    }

    fn public_error(&self, error: &str) -> String {
        error
            .replace(&self.options.api_key, "<redacted>")
            .replace("x-apisports-key", "API-Football key")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LeagueSeasonKey {
    league_id: u64,
    season: u16,
}

fn league_season_key(fixture: &ApiFixture) -> Option<LeagueSeasonKey> {
    Some(LeagueSeasonKey {
        league_id: fixture.league.id?,
        season: fixture.league.season?,
    })
}

fn flatten_standings(responses: Vec<ApiStandingResponse>) -> Vec<ApiStandingRow> {
    responses
        .into_iter()
        .flat_map(|response| response.league.standings)
        .flatten()
        .collect()
}

fn parse_envelope<T>(body: &str) -> Result<Vec<T>, String>
where
    T: serde::de::DeserializeOwned,
{
    let envelope = serde_json::from_str::<ApiFootballEnvelope<T>>(body)
        .map_err(|error| format!("invalid JSON: {error}"))?;
    if envelope.has_errors() {
        return Err(format!("API errors: {}", envelope.error_summary()));
    }
    Ok(envelope.response)
}
