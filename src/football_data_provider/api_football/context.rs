use std::collections::HashMap;

use crate::domain::BetCandidate;
use crate::football_data_provider::append_candidate_note;

use super::models::{ApiFixture, ApiInjury, ApiStandingRow};
use super::time::iso_to_utc_minutes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CandidateFixtureMatch {
    pub(super) candidate_index: usize,
    pub(super) fixture_index: usize,
}

pub(super) fn match_candidates(
    candidates: &[BetCandidate],
    fixtures: &[ApiFixture],
) -> Vec<CandidateFixtureMatch> {
    let mut matches = Vec::new();
    for (candidate_index, candidate) in candidates.iter().enumerate() {
        if let Some((fixture_index, _)) = fixtures
            .iter()
            .enumerate()
            .filter(|(_, fixture)| fixture_matches_candidate(candidate, fixture))
            .min_by_key(|(_, fixture)| kickoff_distance(candidate, fixture).unwrap_or(i64::MAX))
        {
            matches.push(CandidateFixtureMatch {
                candidate_index,
                fixture_index,
            });
        }
    }
    matches
}

pub(super) fn append_fixture_note(candidate: &mut BetCandidate, fixture: &ApiFixture) {
    append_candidate_note(
        candidate,
        format!(
            "API-Football fixture matched: {} vs {} in {} at {}",
            fixture.teams.home.name,
            fixture.teams.away.name,
            fixture.league.name,
            fixture.fixture.date
        ),
    );
}

pub(super) fn append_injury_notes(
    candidate: &mut BetCandidate,
    fixture: &ApiFixture,
    injuries: &[ApiInjury],
) {
    if injuries.is_empty() {
        append_candidate_note(
            candidate,
            format!(
                "API-Football availability checked: no listed absences for fixture {}",
                fixture.fixture.id
            ),
        );
        return;
    }

    let summary = injuries
        .iter()
        .take(4)
        .map(|injury| {
            format!(
                "{} {} {} {}",
                injury.team.name,
                injury.player.name,
                injury.kind.as_deref().unwrap_or("injury"),
                injury.reason.as_deref().unwrap_or("availability concern")
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    append_candidate_note(candidate, format!("API-Football injuries: {summary}"));
}

pub(super) fn append_availability_coverage_note(
    candidate: &mut BetCandidate,
    fixture: &ApiFixture,
    status: &str,
) {
    append_candidate_note(
        candidate,
        format!(
            "API-Football availability coverage {status} for {} {}",
            fixture.league.name,
            fixture
                .league
                .season
                .map_or_else(|| "unknown season".to_string(), |season| season.to_string())
        ),
    );
}

pub(super) fn append_form_notes(
    candidate: &mut BetCandidate,
    fixture: &ApiFixture,
    form_cache: &HashMap<u64, Vec<ApiFixture>>,
) {
    for team in [&fixture.teams.home, &fixture.teams.away] {
        let Some(fixtures) = form_cache.get(&team.id) else {
            continue;
        };
        if let Some(note) = form_note(team.id, &team.name, &fixture.fixture.date, fixtures) {
            append_candidate_note(candidate, note);
        }
    }
}

pub(super) fn append_standings_notes(
    candidate: &mut BetCandidate,
    fixture: &ApiFixture,
    standings: &[ApiStandingRow],
) {
    for team in [&fixture.teams.home, &fixture.teams.away] {
        if let Some(note) = standings_note(team.id, &team.name, standings) {
            append_candidate_note(candidate, note);
        }
    }
}

pub(super) fn append_standings_coverage_note(
    candidate: &mut BetCandidate,
    fixture: &ApiFixture,
    status: &str,
) {
    append_candidate_note(
        candidate,
        format!(
            "API-Football table coverage {status} for {} {}",
            fixture.league.name,
            fixture
                .league
                .season
                .map_or_else(|| "unknown season".to_string(), |season| season.to_string())
        ),
    );
}

fn fixture_matches_candidate(candidate: &BetCandidate, fixture: &ApiFixture) -> bool {
    let event = normalize_name(&candidate.event);
    let home = normalize_name(&fixture.teams.home.name);
    let away = normalize_name(&fixture.teams.away.name);
    event.contains(&home)
        && event.contains(&away)
        && kickoff_distance(candidate, fixture).is_none_or(|minutes| minutes <= 180)
}

fn standings_note(team_id: u64, team_name: &str, standings: &[ApiStandingRow]) -> Option<String> {
    let total = standings.len();
    let row = standings.iter().find(|row| row.team.id == team_id)?;
    let mut drivers = Vec::new();
    if row.rank <= 2 {
        drivers.push("title race");
    }
    if total >= 6 && usize::from(row.rank) >= total.saturating_sub(2) {
        drivers.push("relegation battle");
    }

    let description = row.description.as_deref().unwrap_or_default();
    let description_lower = description.to_lowercase();
    if description_lower.contains("champions league")
        || description_lower.contains("conference league")
        || description_lower.contains("europa")
    {
        drivers.push("europe place");
    } else if description_lower.contains("promotion") {
        drivers.push("promotion path");
    }
    if description_lower.contains("relegation") && !drivers.contains(&"relegation battle") {
        drivers.push("relegation battle");
    }

    if drivers.is_empty() {
        return None;
    }

    let points = row.points.map_or_else(
        || "points n/a".to_string(),
        |points| format!("{points} pts"),
    );
    let goal_diff = row
        .goals_diff
        .map_or_else(String::new, |diff| format!(", goal difference {diff:+}"));
    let form = row
        .form
        .as_deref()
        .filter(|form| !form.trim().is_empty())
        .map_or_else(String::new, |form| format!(", table form {form}"));
    let status = row
        .status
        .as_deref()
        .filter(|status| !status.trim().is_empty() && *status != "same")
        .map_or_else(String::new, |status| format!(", table status {status}"));
    let group = row
        .group
        .as_deref()
        .filter(|group| !group.trim().is_empty())
        .map_or_else(String::new, |group| format!(" ({group})"));
    Some(format!(
        "API-Football motivation: {team_name}{group} {}; rank {}/{total}, {points}{goal_diff}{form}{status}",
        drivers.join(" and "),
        row.rank
    ))
}

fn kickoff_distance(candidate: &BetCandidate, fixture: &ApiFixture) -> Option<i64> {
    let candidate_minutes = iso_to_utc_minutes(&candidate.starts_at)?;
    let fixture_minutes = iso_to_utc_minutes(&fixture.fixture.date)?;
    Some((candidate_minutes - fixture_minutes).abs())
}

fn form_note(
    team_id: u64,
    team_name: &str,
    fixture_date: &str,
    fixtures: &[ApiFixture],
) -> Option<String> {
    let mut results = Vec::new();
    let mut last_played = None;
    for fixture in fixtures {
        if fixture.fixture.date.as_str() >= fixture_date {
            continue;
        }
        let Some(result) = result_for_team(team_id, fixture) else {
            continue;
        };
        results.push(result);
        if let Some(minutes) = iso_to_utc_minutes(&fixture.fixture.date)
            && last_played.is_none_or(|(last_minutes, _)| minutes > last_minutes)
        {
            last_played = Some((minutes, fixture.fixture.date.as_str()));
        }
    }
    if results.is_empty() {
        return None;
    }

    let wins = results.iter().filter(|result| **result == 'W').count();
    let losses = results.iter().filter(|result| **result == 'L').count();
    let form_label = if wins >= 3 {
        "good form"
    } else if losses >= 3 || wins == 0 {
        "poor form"
    } else {
        "mixed recent form"
    };
    let mut note = format!(
        "API-Football form: {team_name} recent form {}; {form_label}",
        results.iter().copied().collect::<String>()
    );
    if let Some(last_played) = last_played.and_then(|(_, date)| rest_days(date, fixture_date)) {
        if last_played <= 3 {
            note.push_str("; short rest");
        } else if last_played >= 6 {
            note.push_str("; full week rest");
        }
    }
    Some(note)
}

fn result_for_team(team_id: u64, fixture: &ApiFixture) -> Option<char> {
    let goals = fixture.goals.as_ref()?;
    let home_goals = goals.home?;
    let away_goals = goals.away?;
    let is_home = fixture.teams.home.id == team_id;
    let is_away = fixture.teams.away.id == team_id;
    if !is_home && !is_away {
        return None;
    }
    let (team_goals, opponent_goals) = if is_home {
        (home_goals, away_goals)
    } else {
        (away_goals, home_goals)
    };
    Some(match team_goals.cmp(&opponent_goals) {
        std::cmp::Ordering::Greater => 'W',
        std::cmp::Ordering::Equal => 'D',
        std::cmp::Ordering::Less => 'L',
    })
}

fn rest_days(previous_fixture_date: &str, next_fixture_date: &str) -> Option<i64> {
    let previous = iso_to_utc_minutes(previous_fixture_date)?;
    let next = iso_to_utc_minutes(next_fixture_date)?;
    Some(((next - previous) / 1440).max(0))
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
