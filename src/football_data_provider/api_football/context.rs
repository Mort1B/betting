use std::collections::HashMap;

use crate::domain::BetCandidate;
use crate::football_data_provider::append_candidate_note;
use crate::team_name::{comparable_name, names_match, normalize_tokens};

use super::models::{ApiFixture, ApiInjury, ApiStandingRow};
use super::time::iso_to_utc_minutes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CandidateFixtureMatch {
    pub(super) candidate_index: usize,
    pub(super) fixture_index: usize,
}

pub(super) fn match_candidate_indexes(
    candidates: &[BetCandidate],
    candidate_indexes: &[usize],
    fixtures: &[ApiFixture],
) -> Vec<CandidateFixtureMatch> {
    let mut matches = Vec::new();
    for &candidate_index in candidate_indexes {
        let Some(candidate) = candidates.get(candidate_index) else {
            continue;
        };
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

    let selected_team = selected_team_id(candidate, fixture);
    let mut selected_absences = Vec::new();
    let mut opponent_absences = Vec::new();
    let mut neutral_absences = Vec::new();

    for injury in injuries.iter().take(4) {
        match team_relation(injury.team.id, selected_team) {
            TeamRelation::Selected => selected_absences.push(selected_absence_summary(injury)),
            TeamRelation::Opponent => opponent_absences.push(opponent_absence_summary(injury)),
            TeamRelation::Neutral => neutral_absences.push(selected_absence_summary(injury)),
        }
    }

    if !selected_absences.is_empty() {
        append_candidate_note(
            candidate,
            format!(
                "API-Football selected team absences: {}",
                selected_absences.join("; ")
            ),
        );
    }
    if !opponent_absences.is_empty() {
        append_candidate_note(
            candidate,
            format!(
                "API-Football opponent absences: {}",
                opponent_absences.join("; ")
            ),
        );
    }
    if !neutral_absences.is_empty() {
        append_candidate_note(
            candidate,
            format!("API-Football injuries: {}", neutral_absences.join("; ")),
        );
    }
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
    let selected_team = selected_team_id(candidate, fixture);
    for team in [&fixture.teams.home, &fixture.teams.away] {
        let Some(fixtures) = form_cache.get(&team.id) else {
            continue;
        };
        if let Some(note) = form_note(
            team.id,
            &team.name,
            team_relation(team.id, selected_team),
            &fixture.fixture.date,
            fixtures,
        ) {
            append_candidate_note(candidate, note);
        } else {
            append_candidate_note(
                candidate,
                format!(
                    "API-Football form checked: no completed recent fixture data for {}",
                    team.name
                ),
            );
        }
    }
}

pub(super) fn append_standings_notes(
    candidate: &mut BetCandidate,
    fixture: &ApiFixture,
    standings: &[ApiStandingRow],
) {
    let selected_team = selected_team_id(candidate, fixture);
    for team in [&fixture.teams.home, &fixture.teams.away] {
        if let Some(note) = standings_note(
            team.id,
            &team.name,
            team_relation(team.id, selected_team),
            standings,
        ) {
            append_candidate_note(candidate, note);
        }
    }
    if standings.is_empty() {
        append_candidate_note(
            candidate,
            format!(
                "API-Football table checked: no standings rows returned for {}",
                fixture.league.name
            ),
        );
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
    candidate_event_teams(&candidate.event).is_some_and(|candidate_teams| {
        teams_match(
            &candidate_teams,
            &fixture.teams.home.name,
            &fixture.teams.away.name,
        )
    }) && kickoff_distance(candidate, fixture).is_none_or(|minutes| minutes <= 180)
}

fn teams_match(candidate_teams: &(String, String), home: &str, away: &str) -> bool {
    (names_match(&candidate_teams.0, home) && names_match(&candidate_teams.1, away))
        || (names_match(&candidate_teams.0, away) && names_match(&candidate_teams.1, home))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeamRelation {
    Selected,
    Opponent,
    Neutral,
}

fn team_relation(team_id: u64, selected_team_id: Option<u64>) -> TeamRelation {
    match selected_team_id {
        Some(selected_team_id) if selected_team_id == team_id => TeamRelation::Selected,
        Some(_) => TeamRelation::Opponent,
        None => TeamRelation::Neutral,
    }
}

fn selected_team_id(candidate: &BetCandidate, fixture: &ApiFixture) -> Option<u64> {
    for team in [&fixture.teams.home, &fixture.teams.away] {
        if selection_mentions_team(&candidate.selection, &team.name) {
            return Some(team.id);
        }
    }
    None
}

fn selection_mentions_team(selection: &str, team_name: &str) -> bool {
    let selection = normalize_tokens(&comparable_name(selection));
    let team = normalize_tokens(&comparable_name(team_name));
    if selection.is_empty() || team.is_empty() || selection.len() < team.len() {
        return false;
    }
    selection.windows(team.len()).any(|window| window == team)
}

fn selected_absence_summary(injury: &ApiInjury) -> String {
    format!(
        "{} {} {} {}",
        injury.team.name,
        injury.player.name,
        injury.kind.as_deref().unwrap_or("injury"),
        injury.reason.as_deref().unwrap_or("availability concern")
    )
}

fn opponent_absence_summary(injury: &ApiInjury) -> String {
    format!("{} {}", injury.team.name, injury.player.name)
}

fn candidate_event_teams(event: &str) -> Option<(String, String)> {
    for separator in [" - ", " vs. ", " vs ", " v ", " @ "] {
        if let Some((left, right)) = event.split_once(separator) {
            let left = comparable_name(left);
            let right = comparable_name(right);
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right));
            }
        }
    }
    None
}

fn standings_note(
    team_id: u64,
    team_name: &str,
    relation: TeamRelation,
    standings: &[ApiStandingRow],
) -> Option<String> {
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
    match relation {
        TeamRelation::Selected | TeamRelation::Neutral => Some(format!(
            "API-Football motivation: {team_name}{group} {}; rank {}/{total}, {points}{goal_diff}{form}{status}",
            drivers.join(" and "),
            row.rank
        )),
        TeamRelation::Opponent => Some(format!(
            "API-Football opponent motivation risk: {team_name}{group} rank {}/{total}, {points}{goal_diff}{form}{status}",
            row.rank
        )),
    }
}

fn kickoff_distance(candidate: &BetCandidate, fixture: &ApiFixture) -> Option<i64> {
    let candidate_minutes = iso_to_utc_minutes(&candidate.starts_at)?;
    let fixture_minutes = iso_to_utc_minutes(&fixture.fixture.date)?;
    Some((candidate_minutes - fixture_minutes).abs())
}

fn form_note(
    team_id: u64,
    team_name: &str,
    relation: TeamRelation,
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
    let form_label = form_label(relation, wins, losses);
    let mut note = format!(
        "API-Football form: {team_name} recent form {}; {form_label}",
        results.iter().copied().collect::<String>()
    );
    if let Some(last_played) = last_played.and_then(|(_, date)| rest_days(date, fixture_date)) {
        if last_played <= 3 {
            note.push_str("; short rest");
        } else if last_played >= 6 {
            note.push_str("; full week rest");
        } else {
            note.push_str("; schedule checked no short-rest signal");
        }
    } else {
        note.push_str("; schedule checked no rest-day signal");
    }
    Some(note)
}

fn form_label(relation: TeamRelation, wins: usize, losses: usize) -> &'static str {
    match relation {
        TeamRelation::Selected | TeamRelation::Neutral => {
            if wins >= 3 {
                "good form"
            } else if losses >= 3 || wins == 0 {
                "poor form"
            } else {
                "mixed recent form"
            }
        }
        TeamRelation::Opponent => {
            if wins >= 3 {
                "opponent strong form"
            } else if losses >= 3 || wins == 0 {
                "opponent vulnerable form"
            } else {
                "opponent mixed form"
            }
        }
    }
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
