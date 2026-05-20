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
    pub(super) name: String,
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
