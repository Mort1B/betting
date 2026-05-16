use std::collections::HashMap;
use std::fs;

use crate::domain::BetCandidate;

#[derive(Debug, Clone, Default)]
pub struct ReferenceOddsOptions {
    pub source_path: Option<String>,
}

pub fn apply_reference_odds(
    candidates: Vec<BetCandidate>,
    options: &ReferenceOddsOptions,
) -> Result<Vec<BetCandidate>, String> {
    let Some(path) = options.source_path.as_deref() else {
        return Ok(candidates);
    };
    let rows = load_reference_rows(path)?;
    Ok(enrich_candidates(candidates, &rows))
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
    for candidate in &mut candidates {
        if candidate.reference_odds.is_some() {
            continue;
        }
        let matches = rows
            .iter()
            .filter(|row| row.matches(candidate))
            .collect::<Vec<_>>();
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
struct ReferenceOddsRow {
    candidate_id: Option<String>,
    sport: Option<String>,
    competition: Option<String>,
    event: Option<String>,
    market: Option<String>,
    selection: Option<String>,
    reference_odds: f64,
    source: String,
    notes: Option<String>,
}

impl ReferenceOddsRow {
    fn matches(&self, candidate: &BetCandidate) -> bool {
        if self
            .candidate_id
            .as_deref()
            .is_some_and(|id| id == candidate.id)
        {
            return true;
        }
        if self.candidate_id.is_some() {
            return false;
        }
        required_match(self.event.as_deref(), &candidate.event)
            && required_match(self.market.as_deref(), &candidate.market)
            && required_match(self.selection.as_deref(), &candidate.selection)
            && optional_match(self.sport.as_deref(), &candidate.sport)
            && optional_match(self.competition.as_deref(), &candidate.competition)
    }
}

fn required_match(reference: Option<&str>, candidate_value: &str) -> bool {
    reference.is_some_and(|value| normalize_key(value) == normalize_key(candidate_value))
}

fn optional_match(reference: Option<&str>, candidate_value: &str) -> bool {
    reference
        .map(|value| normalize_key(value) == normalize_key(candidate_value))
        .unwrap_or(true)
}

fn consensus_reference_odds(rows: &[&ReferenceOddsRow]) -> f64 {
    let average_probability =
        rows.iter().map(|row| 1.0 / row.reference_odds).sum::<f64>() / rows.len() as f64;
    round_to_two_decimals(1.0 / average_probability)
}

fn append_reference_note(
    candidate: &mut BetCandidate,
    matches: &[&ReferenceOddsRow],
    reference_odds: f64,
) {
    let sources = matches
        .iter()
        .take(4)
        .map(|row| format!("{} {:.2}", row.source, row.reference_odds))
        .collect::<Vec<_>>()
        .join("; ");
    let mut note = format!(
        "reference odds consensus {:.2} from {} source(s): {}",
        reference_odds,
        matches.len(),
        sources
    );
    let row_notes = matches
        .iter()
        .filter_map(|row| row.notes.as_deref())
        .take(2)
        .collect::<Vec<_>>();
    if !row_notes.is_empty() {
        note.push_str(&format!("; notes: {}", row_notes.join("; ")));
    }

    if candidate.notes.trim().is_empty() {
        candidate.notes = note;
    } else {
        candidate.notes = format!("{}; {}", candidate.notes, note);
    }
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
mod tests {
    use super::*;

    fn candidate() -> BetCandidate {
        BetCandidate {
            id: "nt-123".to_string(),
            sport: "Football".to_string(),
            competition: "Eliteserien".to_string(),
            event: "Rosenborg - Brann".to_string(),
            market: "Main market".to_string(),
            selection: "Rosenborg".to_string(),
            norsk_tipping_odds: 1.22,
            model_probability: None,
            reference_odds: None,
            confidence: Some(0.72),
            starts_at: "2026-05-16T18:00:00+02:00".to_string(),
            notes: "live import".to_string(),
        }
    }

    #[test]
    fn enriches_by_candidate_id_with_consensus_reference_odds() {
        let rows = parse_reference_rows(
            "candidate_id,source,reference_odds,notes\n\
             nt-123,Book A,1.15,market close\n\
             nt-123,Book B,1.17,market close",
        )
        .expect("valid rows");

        let enriched = enrich_candidates(vec![candidate()], &rows);
        assert_eq!(enriched[0].reference_odds, Some(1.16));
        assert!(enriched[0].notes.contains("reference odds consensus 1.16"));
    }

    #[test]
    fn enriches_by_event_market_and_selection() {
        let rows = parse_reference_rows(
            "event,market,selection,reference_odds,source\n\
             \"rosenborg brann\",\"main-market\",Rosenborg,1.16,Book A",
        )
        .expect("valid rows");

        let enriched = enrich_candidates(vec![candidate()], &rows);
        assert_eq!(enriched[0].reference_odds, Some(1.16));
    }

    #[test]
    fn keeps_existing_reference_odds() {
        let rows =
            parse_reference_rows("candidate_id,reference_odds\nnt-123,1.15").expect("valid rows");
        let mut candidate = candidate();
        candidate.reference_odds = Some(1.20);

        let enriched = enrich_candidates(vec![candidate], &rows);
        assert_eq!(enriched[0].reference_odds, Some(1.20));
    }

    #[test]
    fn rejects_rows_without_match_key() {
        let error =
            parse_reference_rows("source,reference_odds\nBook A,1.15").expect_err("invalid row");
        assert!(error.contains("candidate_id"));
    }
}
