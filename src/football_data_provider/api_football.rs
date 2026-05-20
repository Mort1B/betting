mod context;
mod models;
#[cfg(test)]
mod tests;
mod time;

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::domain::{BetCandidate, BettingRules};
use crate::football_data_provider::{FootballContextProvider, FootballDataResult};

use context::{append_fixture_note, append_form_notes, append_injury_notes, match_candidates};
use models::{ApiFixture, ApiFootballEnvelope, ApiInjury};

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
        let fixtures = match self.fetch_fixtures_by_date(&client, date, &mut stats) {
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
        let mut notes = Vec::new();
        let mut fixture_ids_with_injury_fetch = HashSet::new();
        let mut team_form_cache = HashMap::new();

        for candidate_match in matches.iter().take(self.options.max_context_fixtures) {
            let fixture = &fixtures[candidate_match.fixture_index];
            append_fixture_note(&mut candidates[candidate_match.candidate_index], fixture);

            if fixture_ids_with_injury_fetch.insert(fixture.fixture.id) {
                match self.fetch_injuries(&client, fixture.fixture.id, &mut stats) {
                    Ok(injuries) => {
                        append_injury_notes(
                            &mut candidates[candidate_match.candidate_index],
                            fixture,
                            &injuries,
                        );
                    }
                    Err(error) => notes.push(format!(
                        "API-Football injury request failed for fixture {}: {error}",
                        fixture.fixture.id
                    )),
                }
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

#[derive(Debug, Default)]
struct ProviderStats {
    fixture_requests: usize,
    fixture_success: usize,
    injury_requests: usize,
    injury_success: usize,
    form_requests: usize,
    form_success: usize,
}

impl ProviderStats {
    fn summary(&self, matched_candidates: usize, candidate_count: usize) -> String {
        format!(
            "API-Football: fixture requests {}/{}, injury requests {}/{}, form team requests {}/{}, matched {matched_candidates}/{candidate_count} candidate(s)",
            self.fixture_success,
            self.fixture_requests,
            self.injury_success,
            self.injury_requests,
            self.form_success,
            self.form_requests
        )
    }
}
