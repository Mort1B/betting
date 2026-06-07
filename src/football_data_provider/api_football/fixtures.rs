use std::collections::HashSet;

use crate::domain::BetCandidate;
use crate::football_data_provider::append_candidate_note;

use super::{ApiFixture, ApiFootballProvider, ProviderStats, context::CandidateFixtureMatch};

const MAX_FIXTURE_DATES: usize = 3;

pub(super) fn fixture_dates_for_candidate_indexes(
    candidates: &[BetCandidate],
    candidate_indexes: &[usize],
    report_date: &str,
) -> Vec<String> {
    let mut dates = Vec::new();
    push_date(&mut dates, report_date);

    for &candidate_index in candidate_indexes {
        let Some(candidate) = candidates.get(candidate_index) else {
            continue;
        };
        let Some(candidate_date) = candidate.starts_at.get(..10) else {
            continue;
        };
        push_date(&mut dates, candidate_date);
        if dates.len() >= MAX_FIXTURE_DATES {
            break;
        }
    }

    dates
}

pub(super) fn append_unmatched_fixture_notes_for_indexes(
    candidates: &mut [BetCandidate],
    matches: &[CandidateFixtureMatch],
    candidate_indexes: &[usize],
    date_count: usize,
) {
    let matched_candidate_indexes = matches
        .iter()
        .map(|candidate_match| candidate_match.candidate_index)
        .collect::<HashSet<_>>();
    for &index in candidate_indexes {
        let Some(candidate) = candidates.get_mut(index) else {
            continue;
        };
        if !matched_candidate_indexes.contains(&index) {
            append_candidate_note(
                candidate,
                format!(
                    "API-Football fixture not matched: no provider fixture matched teams/start across {date_count} fixture date(s)"
                ),
            );
        }
    }
}

pub(super) fn append_skipped_fixture_notes(
    candidates: &mut [BetCandidate],
    matches: &[CandidateFixtureMatch],
    max_context_fixtures: usize,
) {
    for candidate_match in matches.iter().skip(max_context_fixtures) {
        let Some(candidate) = candidates.get_mut(candidate_match.candidate_index) else {
            continue;
        };
        append_candidate_note(
            candidate,
            format!(
                "API-Football fixture matched but context enrichment skipped: matched fixture cap {max_context_fixtures} reached"
            ),
        );
    }
}

impl ApiFootballProvider {
    pub(super) fn fetch_fixtures_for_dates(
        &self,
        client: &reqwest::blocking::Client,
        dates: &[String],
        stats: &mut ProviderStats,
        notes: &mut Vec<String>,
    ) -> Result<Vec<ApiFixture>, String> {
        let mut fixtures = Vec::new();
        let mut errors = Vec::new();

        for date in dates {
            match self.fetch_fixtures_by_date(client, date, stats) {
                Ok(mut fetched_fixtures) => fixtures.append(&mut fetched_fixtures),
                Err(error) => errors.push(format!("{date}: {error}")),
            }
        }

        if fixtures.is_empty() && !errors.is_empty() {
            return Err(errors.join("; "));
        }

        notes.extend(
            errors
                .into_iter()
                .take(2)
                .map(|error| format!("API-Football fixture request failed for {error}")),
        );
        Ok(fixtures)
    }
}

fn push_date(dates: &mut Vec<String>, value: &str) {
    if dates.len() >= MAX_FIXTURE_DATES || !is_iso_date(value) {
        return;
    }
    if !dates.iter().any(|date| date == value) {
        dates.push(value.to_string());
    }
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, starts_at: &str) -> BetCandidate {
        BetCandidate {
            id: id.to_string(),
            sport: "Football".to_string(),
            competition: "Copa Libertadores".to_string(),
            event: "Home - Away".to_string(),
            market: "Main market".to_string(),
            selection: "Home".to_string(),
            norsk_tipping_odds: 1.22,
            model_probability: None,
            reference_odds: None,
            confidence: Some(0.80),
            starts_at: starts_at.to_string(),
            notes: String::new(),
        }
    }

    #[test]
    fn includes_candidate_overnight_fixture_dates() {
        let candidates = vec![
            candidate("c1", "2026-05-21T02:00:00+02:00"),
            candidate("c2", "2026-05-21T02:30:00+02:00"),
        ];
        let candidate_indexes = (0..candidates.len()).collect::<Vec<_>>();
        let dates =
            fixture_dates_for_candidate_indexes(&candidates, &candidate_indexes, "2026-05-20");

        assert_eq!(
            dates,
            vec!["2026-05-20".to_string(), "2026-05-21".to_string()]
        );
    }

    #[test]
    fn marks_matched_candidates_skipped_by_context_cap() {
        let mut candidates = vec![
            candidate("c1", "2026-05-21T18:00:00+02:00"),
            candidate("c2", "2026-05-21T19:00:00+02:00"),
        ];
        let matches = vec![
            CandidateFixtureMatch {
                candidate_index: 0,
                fixture_index: 0,
            },
            CandidateFixtureMatch {
                candidate_index: 1,
                fixture_index: 1,
            },
        ];

        append_skipped_fixture_notes(&mut candidates, &matches, 1);

        assert!(!candidates[0].notes.contains("context enrichment skipped"));
        assert!(
            candidates[1]
                .notes
                .contains("context enrichment skipped: matched fixture cap 1 reached")
        );
    }
}
