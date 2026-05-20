use crate::domain::BetCandidate;

use super::{ReferenceOddsRow, round_to_two_decimals};

const HIGH_MARKET_DISAGREEMENT_RATIO: f64 = 0.05;

pub(super) fn consensus_reference_odds(rows: &[&ReferenceOddsRow]) -> f64 {
    let average_probability =
        rows.iter().map(|row| 1.0 / row.reference_odds).sum::<f64>() / rows.len() as f64;
    round_to_two_decimals(1.0 / average_probability)
}

pub(super) fn append_reference_note(
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
    let context = reference_market_context(matches);
    let mut note = format!(
        "reference odds consensus {:.2} from {} source(s): {}; {}",
        reference_odds,
        matches.len(),
        sources,
        context
    );
    let row_notes = matches
        .iter()
        .filter_map(|row| row.notes.as_deref())
        .take(4)
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

fn reference_market_context(matches: &[&ReferenceOddsRow]) -> String {
    let mut odds = matches
        .iter()
        .map(|row| row.reference_odds)
        .collect::<Vec<_>>();
    odds.sort_by(f64::total_cmp);
    let worst = odds.first().copied().unwrap_or(0.0);
    let best = odds.last().copied().unwrap_or(0.0);
    let average = odds.iter().sum::<f64>() / odds.len() as f64;
    let spread = if worst > 0.0 {
        (best / worst) - 1.0
    } else {
        0.0
    };
    let disagreement = if matches.len() > 1 && spread >= HIGH_MARKET_DISAGREEMENT_RATIO {
        "market disagreement high"
    } else if matches.len() > 1 {
        "market agreement tight"
    } else {
        "single reference source"
    };

    format!(
        "bookmaker/source count {}; range {:.2}-{:.2}; average {:.2}; spread {:.1}%; {}",
        matches.len(),
        worst,
        best,
        round_to_two_decimals(average),
        spread * 100.0,
        disagreement
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_market_disagreement_when_reference_range_is_wide() {
        let rows = [
            ReferenceOddsRow {
                candidate_id: Some("pick".to_string()),
                sport: None,
                competition: None,
                event: None,
                market: None,
                selection: None,
                reference_odds: 1.10,
                source: "Book A".to_string(),
                notes: None,
            },
            ReferenceOddsRow {
                candidate_id: Some("pick".to_string()),
                sport: None,
                competition: None,
                event: None,
                market: None,
                selection: None,
                reference_odds: 1.20,
                source: "Book B".to_string(),
                notes: None,
            },
        ];
        let refs = rows.iter().collect::<Vec<_>>();

        let context = reference_market_context(&refs);

        assert!(context.contains("range 1.10-1.20"));
        assert!(context.contains("market disagreement high"));
    }
}
