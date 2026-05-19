use std::fs;

use serde::Deserialize;

use crate::domain::SportScope;
use crate::history::{HistoryKey, PickHistoryEntry, ResultStatus};

#[derive(Debug, Clone)]
pub(crate) struct SettlementRecords {
    records: Vec<SettlementRecord>,
}

#[derive(Debug, Clone, Deserialize)]
struct SettlementRecord {
    report_date: String,
    event: String,
    market: String,
    selection: String,
    starts_at: String,
    candidate_id: Option<String>,
    result_status: ResultStatus,
    #[serde(alias = "source")]
    settlement_source: String,
    #[serde(default, alias = "source_url")]
    settlement_source_url: Option<String>,
    settled_at: String,
}

impl SettlementRecords {
    pub(crate) fn read(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
        let records = parse_settlements(&content).map_err(|error| format!("{path}: {error}"))?;
        Ok(Self { records })
    }

    pub(crate) fn apply_to(&self, entries: &mut [PickHistoryEntry]) -> usize {
        apply_settlements(entries, &self.records)
    }
}

fn parse_settlements(content: &str) -> Result<Vec<SettlementRecord>, String> {
    let mut settlements = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let record = serde_json::from_str::<SettlementRecord>(trimmed)
            .map_err(|error| format!("line {}: {error}", index + 1))?;
        record
            .validate()
            .map_err(|error| format!("line {}: {error}", index + 1))?;
        settlements.push(record);
    }
    Ok(settlements)
}

fn apply_settlements(entries: &mut [PickHistoryEntry], settlements: &[SettlementRecord]) -> usize {
    let mut updated = 0;
    for settlement in settlements {
        for entry in entries.iter_mut().filter(|entry| settlement.matches(entry)) {
            if !SportScope::Football.allows_sport(&entry.sport) {
                continue;
            }
            if entry.result_status.is_settled() {
                continue;
            }
            entry.result_status = settlement.result_status;
            entry.settlement_source = Some(settlement.settlement_source.trim().to_string());
            entry.settlement_source_url = non_empty_option(&settlement.settlement_source_url);
            entry.settled_at = Some(settlement.settled_at.trim().to_string());
            updated += 1;
        }
    }
    updated
}

impl SettlementRecord {
    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("report_date", &self.report_date),
            ("event", &self.event),
            ("market", &self.market),
            ("selection", &self.selection),
            ("starts_at", &self.starts_at),
            ("settlement_source", &self.settlement_source),
            ("settled_at", &self.settled_at),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{name} must not be empty"));
            }
        }
        if self.result_status == ResultStatus::Pending {
            return Err("result_status must be win, loss, void, or unknown".to_string());
        }
        Ok(())
    }

    fn matches(&self, entry: &PickHistoryEntry) -> bool {
        if self
            .candidate_id
            .as_deref()
            .is_some_and(|id| id.trim() != entry.candidate_id)
        {
            return false;
        }
        entry.key() == self.key()
    }

    fn key(&self) -> HistoryKey {
        HistoryKey {
            report_date: self.report_date.trim().to_string(),
            event: self.event.trim().to_string(),
            market: self.market.trim().to_string(),
            selection: self.selection.trim().to_string(),
            starts_at: self.starts_at.trim().to_string(),
        }
    }
}

fn non_empty_option(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_verified_win_to_pending_entry() {
        let mut entries = vec![entry(ResultStatus::Pending)];
        let settlements = parse_settlements(
            r#"{"report_date":"2026-05-15","candidate_id":"ex-001","event":"Rosenborg - Brann","market":"Double chance","selection":"Rosenborg or draw","starts_at":"2026-05-15T18:00:00+02:00","result_status":"win","settlement_source":"manual final score","settlement_source_url":"https://example.com/match","settled_at":"2026-05-16T10:00:00Z"}"#,
        )
        .expect("valid settlements");

        assert_eq!(apply_settlements(&mut entries, &settlements), 1);
        assert_eq!(entries[0].result_status, ResultStatus::Win);
        assert_eq!(
            entries[0].settlement_source.as_deref(),
            Some("manual final score")
        );
        assert_eq!(
            entries[0].settlement_source_url.as_deref(),
            Some("https://example.com/match")
        );
    }

    #[test]
    fn preserves_already_settled_entries() {
        let mut entries = vec![entry(ResultStatus::Loss)];
        let settlements = parse_settlements(
            r#"{"report_date":"2026-05-15","event":"Rosenborg - Brann","market":"Double chance","selection":"Rosenborg or draw","starts_at":"2026-05-15T18:00:00+02:00","result_status":"unknown","settlement_source":"manual check","settled_at":"2026-05-16T10:00:00Z"}"#,
        )
        .expect("valid settlements");

        assert_eq!(apply_settlements(&mut entries, &settlements), 0);
        assert_eq!(entries[0].result_status, ResultStatus::Loss);
    }

    #[test]
    fn rejects_pending_settlement_records() {
        let error = parse_settlements(
            r#"{"report_date":"2026-05-15","event":"Rosenborg - Brann","market":"Double chance","selection":"Rosenborg or draw","starts_at":"2026-05-15T18:00:00+02:00","result_status":"pending","settlement_source":"manual check","settled_at":"2026-05-16T10:00:00Z"}"#,
        )
        .expect_err("invalid settlement");

        assert!(error.contains("win, loss, void, or unknown"));
    }

    #[test]
    fn candidate_id_mismatch_does_not_update() {
        let mut entries = vec![entry(ResultStatus::Pending)];
        let settlements = parse_settlements(
            r#"{"report_date":"2026-05-15","candidate_id":"other","event":"Rosenborg - Brann","market":"Double chance","selection":"Rosenborg or draw","starts_at":"2026-05-15T18:00:00+02:00","result_status":"void","settlement_source":"postponed list","settled_at":"2026-05-16T10:00:00Z"}"#,
        )
        .expect("valid settlements");

        assert_eq!(apply_settlements(&mut entries, &settlements), 0);
        assert_eq!(entries[0].result_status, ResultStatus::Pending);
    }

    #[test]
    fn ignores_non_football_history_entries() {
        let mut entries = vec![PickHistoryEntry {
            sport: "Tennis".to_string(),
            ..entry(ResultStatus::Pending)
        }];
        let settlements = parse_settlements(
            r#"{"report_date":"2026-05-15","event":"Rosenborg - Brann","market":"Double chance","selection":"Rosenborg or draw","starts_at":"2026-05-15T18:00:00+02:00","result_status":"win","settlement_source":"manual check","settled_at":"2026-05-16T10:00:00Z"}"#,
        )
        .expect("valid settlements");

        assert_eq!(apply_settlements(&mut entries, &settlements), 0);
        assert_eq!(entries[0].result_status, ResultStatus::Pending);
    }

    fn entry(result_status: ResultStatus) -> PickHistoryEntry {
        PickHistoryEntry {
            report_date: "2026-05-15".to_string(),
            rank: 1,
            candidate_id: "ex-001".to_string(),
            sport: "Football".to_string(),
            competition: "Eliteserien".to_string(),
            event: "Rosenborg - Brann".to_string(),
            market: "Double chance".to_string(),
            selection: "Rosenborg or draw".to_string(),
            starts_at: "2026-05-15T18:00:00+02:00".to_string(),
            norsk_tipping_odds: 1.22,
            score: 80.0,
            confidence: 0.75,
            strict_status: "pass".to_string(),
            rejection_reasons: Vec::new(),
            football_context: Vec::new(),
            result_status,
            settlement_source: None,
            settlement_source_url: None,
            settled_at: None,
        }
    }
}
