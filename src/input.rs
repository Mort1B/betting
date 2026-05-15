use std::collections::HashMap;
use std::fs;

use crate::domain::BetCandidate;

pub fn load_candidates_from_csv(path: &str) -> Result<Vec<BetCandidate>, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    parse_candidates_csv(&content)
}

pub fn parse_candidates_csv(content: &str) -> Result<Vec<BetCandidate>, String> {
    let mut rows = content.lines().enumerate().filter(|(_, line)| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    });

    let (header_line_number, header_line) = rows
        .next()
        .ok_or_else(|| "CSV file is empty or contains only comments".to_string())?;
    let headers = split_csv_line(header_line)
        .map_err(|error| format!("line {}: {error}", header_line_number + 1))?;
    let header_index = index_headers(&headers);

    let mut candidates = Vec::new();
    for (line_number, line) in rows {
        let row =
            split_csv_line(line).map_err(|error| format!("line {}: {error}", line_number + 1))?;
        let candidate = parse_row(&header_index, &row)
            .map_err(|error| format!("line {}: {error}", line_number + 1))?;
        candidates.push(candidate);
    }

    if candidates.is_empty() {
        return Err("CSV did not contain any candidates".to_string());
    }

    Ok(candidates)
}

fn parse_row(headers: &HashMap<String, usize>, row: &[String]) -> Result<BetCandidate, String> {
    let id = required_string(headers, row, "id")?;
    let sport = required_string(headers, row, "sport")?;
    let competition = required_string(headers, row, "competition")?;
    let event = required_string(headers, row, "event")?;
    let market = required_string(headers, row, "market")?;
    let selection = required_string(headers, row, "selection")?;
    let norsk_tipping_odds = required_f64(
        headers,
        row,
        &["norsk_tipping_odds", "odds"],
        "norsk_tipping_odds",
    )?;
    let model_probability = optional_f64(headers, row, "model_probability")?;
    let reference_odds = optional_f64(headers, row, "reference_odds")?;
    let confidence = optional_f64(headers, row, "confidence")?;
    let starts_at = required_string(headers, row, "starts_at")?;
    let notes = optional_string(headers, row, "notes").unwrap_or_default();

    if norsk_tipping_odds <= 1.0 {
        return Err(format!(
            "norsk_tipping_odds must be greater than 1.0 for {id}"
        ));
    }
    validate_probability(model_probability, "model_probability", &id)?;
    validate_probability(confidence, "confidence", &id)?;
    if let Some(reference_odds) = reference_odds
        && reference_odds <= 1.0
    {
        return Err(format!("reference_odds must be greater than 1.0 for {id}"));
    }

    Ok(BetCandidate {
        id,
        sport,
        competition,
        event,
        market,
        selection,
        norsk_tipping_odds,
        model_probability,
        reference_odds,
        confidence,
        starts_at,
        notes,
    })
}

fn required_string(
    headers: &HashMap<String, usize>,
    row: &[String],
    header: &str,
) -> Result<String, String> {
    optional_string(headers, row, header)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing required value for {header}"))
}

fn optional_string(
    headers: &HashMap<String, usize>,
    row: &[String],
    header: &str,
) -> Option<String> {
    let index = headers.get(header)?;
    row.get(*index).map(|value| value.trim().to_string())
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

fn optional_f64(
    headers: &HashMap<String, usize>,
    row: &[String],
    header: &str,
) -> Result<Option<f64>, String> {
    let Some(index) = headers.get(header) else {
        return Ok(None);
    };
    let raw = row.get(*index).map(String::as_str).unwrap_or("").trim();
    if raw.is_empty() {
        return Ok(None);
    }
    raw.parse::<f64>()
        .map(Some)
        .map_err(|_| format!("{header} must be numeric, got {raw}"))
}

fn validate_probability(value: Option<f64>, field: &str, id: &str) -> Result<(), String> {
    if let Some(value) = value
        && !(0.0..=1.0).contains(&value)
    {
        return Err(format!("{field} must be between 0.0 and 1.0 for {id}"));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_candidates() {
        let csv = "id,sport,competition,event,market,selection,norsk_tipping_odds,model_probability,reference_odds,confidence,starts_at,notes\n\
                   t1,Football,Eliteserien,\"Home, Away\",Winner,Home,1.22,0.86,1.18,0.72,2026-05-15T18:00:00+02:00,\"lineup checked\"";

        let parsed = parse_candidates_csv(csv).expect("valid CSV");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].event, "Home, Away");
        assert_eq!(parsed[0].norsk_tipping_odds, 1.22);
    }
}
