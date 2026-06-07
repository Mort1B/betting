use std::collections::HashMap;
use std::fs;

use crate::domain::BetCandidate;
use crate::reference_provider::{ReferenceProviderOptions, fetch_reference_provider_rows};

mod notes;

use notes::{append_reference_note, consensus_reference_odds};

#[derive(Debug, Clone, Default)]
pub struct ReferenceOddsOptions {
    pub source_path: Option<String>,
    pub providers: ReferenceProviderOptions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceOddsResult {
    pub candidates: Vec<BetCandidate>,
    pub provider_report_notes: Vec<String>,
}

pub fn apply_reference_odds(
    candidates: Vec<BetCandidate>,
    options: &ReferenceOddsOptions,
) -> Result<ReferenceOddsResult, String> {
    if candidates.is_empty() {
        return Ok(ReferenceOddsResult {
            candidates,
            provider_report_notes: no_candidate_provider_notes(options),
        });
    }

    let mut rows = Vec::new();
    if let Some(path) = options.source_path.as_deref() {
        rows.extend(load_reference_rows(path)?);
    }

    let provider_output = fetch_reference_provider_rows(&candidates, &options.providers);
    let provider_report_notes =
        provider_report_notes(provider_output.summaries, provider_output.notes);
    rows.extend(provider_output.rows);

    if rows.is_empty() {
        return Ok(ReferenceOddsResult {
            candidates,
            provider_report_notes,
        });
    }

    Ok(ReferenceOddsResult {
        candidates: enrich_candidates(candidates, &rows),
        provider_report_notes,
    })
}

fn no_candidate_provider_notes(options: &ReferenceOddsOptions) -> Vec<String> {
    if options.source_path.is_none() && !has_configured_reference_provider(&options.providers) {
        return Vec::new();
    }

    vec!["Reference odds skipped: no Norsk Tipping candidates to enrich".to_string()]
}

fn has_configured_reference_provider(options: &ReferenceProviderOptions) -> bool {
    options.the_odds_api.is_some()
}

fn load_reference_rows(path: &str) -> Result<Vec<ReferenceOddsRow>, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    parse_reference_rows(&content)
}

fn parse_reference_rows(content: &str) -> Result<Vec<ReferenceOddsRow>, String> {
    let mut rows = content.lines().enumerate().filter(|(_, line)| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    });

    let (header_line_number, header_line) = rows
        .next()
        .ok_or_else(|| "reference odds CSV is empty or contains only comments".to_string())?;
    let headers = split_csv_line(header_line)
        .map_err(|error| format!("line {}: {error}", header_line_number + 1))?;
    let header_index = index_headers(&headers);

    let mut parsed = Vec::new();
    for (line_number, line) in rows {
        let row =
            split_csv_line(line).map_err(|error| format!("line {}: {error}", line_number + 1))?;
        parsed.push(
            parse_row(&header_index, &row)
                .map_err(|error| format!("line {}: {error}", line_number + 1))?,
        );
    }

    if parsed.is_empty() {
        return Err("reference odds CSV did not contain any rows".to_string());
    }
    Ok(parsed)
}

fn enrich_candidates(
    mut candidates: Vec<BetCandidate>,
    rows: &[ReferenceOddsRow],
) -> Vec<BetCandidate> {
    let index = ReferenceOddsIndex::build(rows);
    for candidate in &mut candidates {
        if candidate.reference_odds.is_some() {
            continue;
        }
        let matches = index.matches(rows, candidate);
        if matches.is_empty() {
            continue;
        }
        let reference_odds = consensus_reference_odds(&matches);
        candidate.reference_odds = Some(reference_odds);
        append_reference_note(candidate, &matches, reference_odds);
    }
    candidates
}

fn parse_row(headers: &HashMap<String, usize>, row: &[String]) -> Result<ReferenceOddsRow, String> {
    let reference_odds = required_f64(headers, row, &["reference_odds", "odds"], "reference_odds")?;
    if reference_odds <= 1.0 {
        return Err(format!(
            "reference_odds must be greater than 1.0, got {reference_odds}"
        ));
    }

    let candidate_id = optional_string(headers, row, "candidate_id");
    let event = optional_string(headers, row, "event");
    let market = optional_string(headers, row, "market");
    let selection = optional_string(headers, row, "selection");
    if candidate_id.is_none() && (event.is_none() || market.is_none() || selection.is_none()) {
        return Err(
            "row must include candidate_id or all of event, market, and selection".to_string(),
        );
    }

    Ok(ReferenceOddsRow {
        candidate_id,
        sport: optional_string(headers, row, "sport"),
        competition: optional_string(headers, row, "competition"),
        event,
        market,
        selection,
        reference_odds,
        source: optional_string(headers, row, "source").unwrap_or_else(|| "reference".to_string()),
        notes: optional_string(headers, row, "notes"),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReferenceOddsRow {
    pub(crate) candidate_id: Option<String>,
    pub(crate) sport: Option<String>,
    pub(crate) competition: Option<String>,
    pub(crate) event: Option<String>,
    pub(crate) market: Option<String>,
    pub(crate) selection: Option<String>,
    pub(crate) reference_odds: f64,
    pub(crate) source: String,
    pub(crate) notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ReferenceOddsIndex {
    by_candidate_id: HashMap<String, Vec<usize>>,
    by_match_key: HashMap<ReferenceMatchKey, Vec<usize>>,
}

impl ReferenceOddsIndex {
    fn build(rows: &[ReferenceOddsRow]) -> Self {
        let mut index = Self::default();
        for (row_index, row) in rows.iter().enumerate() {
            if let Some(candidate_id) = row.candidate_id.as_deref() {
                index
                    .by_candidate_id
                    .entry(candidate_id.to_string())
                    .or_default()
                    .push(row_index);
                continue;
            }

            if let Some(key) = row.match_key() {
                index.by_match_key.entry(key).or_default().push(row_index);
            }
        }
        index
    }

    fn matches<'a>(
        &self,
        rows: &'a [ReferenceOddsRow],
        candidate: &BetCandidate,
    ) -> Vec<&'a ReferenceOddsRow> {
        let mut row_indexes = Vec::new();
        if let Some(indexes) = self.by_candidate_id.get(&candidate.id) {
            row_indexes.extend(indexes.iter().copied());
        }

        let key = ReferenceMatchKey::from_parts(
            &candidate.event,
            &candidate.market,
            &candidate.selection,
        );
        let sport = normalize_key(&candidate.sport);
        let competition = normalize_key(&candidate.competition);
        if let Some(indexes) = self.by_match_key.get(&key) {
            row_indexes.extend(indexes.iter().copied().filter(|row_index| {
                rows[*row_index].matches_optional_constraints(&sport, &competition)
            }));
        }

        row_indexes.sort_unstable();
        row_indexes.dedup();
        row_indexes
            .into_iter()
            .map(|row_index| &rows[row_index])
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ReferenceMatchKey {
    event: String,
    market: String,
    selection: String,
}

impl ReferenceMatchKey {
    fn from_parts(event: &str, market: &str, selection: &str) -> Self {
        Self {
            event: normalize_key(event),
            market: normalize_key(market),
            selection: normalize_key(selection),
        }
    }
}

impl ReferenceOddsRow {
    fn match_key(&self) -> Option<ReferenceMatchKey> {
        Some(ReferenceMatchKey::from_parts(
            self.event.as_deref()?,
            self.market.as_deref()?,
            self.selection.as_deref()?,
        ))
    }

    fn matches_optional_constraints(&self, sport: &str, competition: &str) -> bool {
        optional_normalized_match(self.sport.as_deref(), sport)
            && optional_normalized_match(self.competition.as_deref(), competition)
    }
}

fn optional_normalized_match(reference: Option<&str>, normalized_candidate_value: &str) -> bool {
    reference.is_none_or(|value| normalize_key(value) == normalized_candidate_value)
}

fn provider_report_notes(mut summaries: Vec<String>, mut notes: Vec<String>) -> Vec<String> {
    summaries.append(&mut notes);
    summaries
}

fn normalize_key(value: &str) -> String {
    let lowered = value
        .chars()
        .flat_map(char::to_lowercase)
        .collect::<String>();
    lowered
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn required_f64(
    headers: &HashMap<String, usize>,
    row: &[String],
    aliases: &[&str],
    display_name: &str,
) -> Result<f64, String> {
    for alias in aliases {
        if let Some(index) = headers.get(*alias) {
            let raw = row.get(*index).map(String::as_str).unwrap_or("").trim();
            if raw.is_empty() {
                return Err(format!("missing required value for {display_name}"));
            }
            return raw
                .parse::<f64>()
                .map_err(|_| format!("{display_name} must be numeric, got {raw}"));
        }
    }
    Err(format!("missing required column {display_name}"))
}

fn optional_string(
    headers: &HashMap<String, usize>,
    row: &[String],
    header: &str,
) -> Option<String> {
    let index = headers.get(header)?;
    row.get(*index)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn index_headers(headers: &[String]) -> HashMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| (header.trim().to_lowercase(), index))
        .collect()
}

fn split_csv_line(line: &str) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err("unterminated quoted field".to_string());
    }
    fields.push(current.trim().to_string());
    Ok(fields)
}

fn round_to_two_decimals(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests;
