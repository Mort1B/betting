use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(super) struct ApiFootballEnvelope<T> {
    pub(super) response: Vec<T>,
    #[serde(default)]
    errors: Value,
}

impl<T> ApiFootballEnvelope<T> {
    pub(super) fn has_errors(&self) -> bool {
        match &self.errors {
            Value::Null => false,
            Value::Array(values) => !values.is_empty(),
            Value::Object(values) => !values.is_empty(),
            Value::String(value) => !value.trim().is_empty(),
            _ => true,
        }
    }

    pub(super) fn error_summary(&self) -> String {
        match &self.errors {
            Value::Object(values) => values
                .iter()
                .map(|(key, value)| format!("{key}: {}", value.as_str().unwrap_or("error")))
                .collect::<Vec<_>>()
                .join("; "),
            Value::Array(values) => values
                .iter()
                .map(|value| value.as_str().unwrap_or("error").to_string())
                .collect::<Vec<_>>()
                .join("; "),
            Value::String(value) => value.to_string(),
            other => other.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct ApiFixture {
    pub(super) fixture: FixtureInfo,
    pub(super) league: LeagueInfo,
    pub(super) teams: FixtureTeams,
    #[serde(default)]
    pub(super) goals: Option<Goals>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct FixtureInfo {
    pub(super) id: u64,
    pub(super) date: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct LeagueInfo {
    #[serde(default)]
    pub(super) id: Option<u64>,
    pub(super) name: String,
    #[serde(default)]
    pub(super) season: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct FixtureTeams {
    pub(super) home: TeamInfo,
    pub(super) away: TeamInfo,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct TeamInfo {
    pub(super) id: u64,
    pub(super) name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct Goals {
    pub(super) home: Option<i32>,
    pub(super) away: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct ApiInjury {
    pub(super) player: InjuryPlayer,
    pub(super) team: TeamInfo,
    #[serde(rename = "type")]
    pub(super) kind: Option<String>,
    #[serde(default)]
    pub(super) reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct InjuryPlayer {
    pub(super) name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct ApiLeagueCoverageResponse {
    pub(super) league: ApiLeagueIdentity,
    #[serde(default)]
    pub(super) seasons: Vec<ApiLeagueSeason>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct ApiLeagueIdentity {
    pub(super) id: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct ApiLeagueSeason {
    pub(super) year: u16,
    #[serde(default)]
    pub(super) coverage: ApiLeagueCoverage,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub(super) struct ApiLeagueCoverage {
    #[serde(default)]
    pub(super) standings: Option<bool>,
    #[serde(default)]
    pub(super) injuries: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct ApiStandingResponse {
    pub(super) league: ApiStandingLeague,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct ApiStandingLeague {
    #[serde(default)]
    pub(super) standings: Vec<Vec<ApiStandingRow>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(super) struct ApiStandingRow {
    pub(super) rank: u16,
    pub(super) team: TeamInfo,
    #[serde(default)]
    pub(super) points: Option<i32>,
    #[serde(default, rename = "goalsDiff")]
    pub(super) goals_diff: Option<i32>,
    #[serde(default)]
    pub(super) group: Option<String>,
    #[serde(default)]
    pub(super) form: Option<String>,
    #[serde(default)]
    pub(super) status: Option<String>,
    #[serde(default)]
    pub(super) description: Option<String>,
}
