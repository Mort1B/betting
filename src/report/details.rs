use crate::domain::EvaluatedCandidate;

pub(super) fn push_ranked_candidate_details(
    output: &mut String,
    rank: usize,
    candidate: &EvaluatedCandidate,
) {
    output.push_str(&format!("#{} {}\n", rank, candidate.candidate.event));
    push_candidate_details(output, candidate);
}

fn push_candidate_details(output: &mut String, candidate: &EvaluatedCandidate) {
    output.push_str(&format!("Sport: {}\n", candidate.candidate.sport));
    output.push_str(&format!("Event: {}\n", candidate.candidate.event));
    output.push_str(&format!(
        "Competition: {}\n",
        candidate.candidate.competition
    ));
    output.push_str(&format!("Starts at: {}\n", candidate.candidate.starts_at));
    output.push_str(&format!("Market: {}\n", candidate.candidate.market));
    output.push_str(&format!("Selection: {}\n", candidate.candidate.selection));
    output.push_str(&format!(
        "Norsk Tipping odds: {:.2}\n",
        candidate.candidate.norsk_tipping_odds
    ));
    if let Some(reference_odds) = candidate.candidate.reference_odds {
        output.push_str(&format!("Reference market odds: {reference_odds:.2}\n"));
        output.push_str(&format!(
            "Norsk Tipping comparison: {}\n",
            price_comparison(candidate.candidate.norsk_tipping_odds, reference_odds)
        ));
    } else {
        output.push_str("Reference market odds: not used\n");
    }
    output.push_str(&format!(
        "Estimated probability: {:.2}%\n",
        candidate.probability.estimated_probability * 100.0
    ));
    output.push_str(&format!(
        "Norsk Tipping implied probability: {:.2}%\n",
        candidate.probability.implied_probability * 100.0
    ));
    output.push_str(&format!(
        "Expected value: {:.2}%\n",
        clean_zero(candidate.value.expected_value * 100.0)
    ));
    output.push_str(&format!(
        "Edge: {:.2} pp\n",
        clean_zero(candidate.value.edge * 100.0)
    ));
    output.push_str(&format!(
        "Confidence: {:.2}%\n",
        candidate.risk.confidence * 100.0
    ));
    output.push_str(&format!(
        "Confidence score: {}/100\n",
        (candidate.risk.confidence * 100.0).round() as u32
    ));
    if candidate.is_bettable() {
        output.push_str("Strict rules status: pass\n");
    } else {
        output.push_str(&format!(
            "Strict rules status: fallback candidate ({})\n",
            candidate.rejection_reasons.join("; ")
        ));
    }
    push_candidate_context(output, candidate);
    output.push_str(&format!("Score: {:.1}\n", candidate.score));
    output.push_str(&format!(
        "Probability sources: {}\n",
        candidate.probability.sources.join(", ")
    ));
    if !candidate.probability.notes.is_empty() {
        output.push_str(&format!(
            "Probability notes: {}\n",
            candidate.probability.notes.join("; ")
        ));
    }
    output.push_str("Explanation: ");
    output.push_str(&candidate_explanation(candidate));
    output.push('\n');
    push_candidate_notes(output, candidate);
}

fn push_candidate_context(output: &mut String, candidate: &EvaluatedCandidate) {
    output.push_str(&format!(
        "Research: reviewed {} page(s), matched {}, positive {}, warnings {}\n",
        candidate.research.pages_reviewed,
        candidate.research.matched_pages,
        candidate.research.positive_mentions,
        candidate.research.warning_mentions
    ));
    output.push_str(&format!(
        "Football context: matched {} page(s), adjustment {:+.2} pp\n",
        candidate.football_context.matched_pages,
        candidate.football_context.confidence_adjustment * 100.0
    ));
    output.push_str(&format!(
        "Learning: {}\n",
        candidate.learning.notes.join("; ")
    ));
    output.push_str("Football context checklist:\n");
    for category in &candidate.football_context.categories {
        let evidence = if category.evidence.is_empty() {
            String::new()
        } else {
            format!(": {}", category.evidence.join("; "))
        };
        output.push_str(&format!(
            "- {}: {}{}\n",
            category.name,
            category.status.label(),
            evidence
        ));
    }
}

fn push_candidate_notes(output: &mut String, candidate: &EvaluatedCandidate) {
    if !candidate.risk.flags.is_empty() {
        output.push_str(&format!(
            "Risk flags: {}\n",
            candidate.risk.flags.join("; ")
        ));
    }
    if !candidate.candidate.notes.is_empty() {
        output.push_str(&format!("Notes: {}\n", candidate.candidate.notes));
    }
    if !candidate.research.price_hints.is_empty() {
        output.push_str("Research price hints:\n");
        for hint in candidate.research.price_hints.iter().take(5) {
            output.push_str(&format!("- {hint}\n"));
        }
    }
    if !candidate.research.notes.is_empty() {
        output.push_str("Research notes:\n");
        for note in candidate.research.notes.iter().take(8) {
            output.push_str(&format!("- {note}\n"));
        }
    }
}

fn candidate_explanation(candidate: &EvaluatedCandidate) -> String {
    let mut parts = vec![format!(
        "The pick clears the {:.2} Norsk Tipping price with {:.2}% estimated probability versus {:.2}% implied probability",
        candidate.candidate.norsk_tipping_odds,
        candidate.probability.estimated_probability * 100.0,
        candidate.probability.implied_probability * 100.0
    )];

    if let Some(reference_odds) = candidate.candidate.reference_odds {
        parts.push(format!(
            "Norsk Tipping is {}",
            price_comparison(candidate.candidate.norsk_tipping_odds, reference_odds)
        ));
    } else {
        parts.push(
            "no reference market odds were used, so ranking relies on market-implied probability, context risk, and research signals"
                .to_string(),
        );
    }

    parts.push(format!(
        "edge is {:.2} pp and expected value is {:.2}%",
        clean_zero(candidate.value.edge * 100.0),
        clean_zero(candidate.value.expected_value * 100.0)
    ));

    parts.push(format!(
        "confidence is {:.2}% after risk and research adjustments",
        candidate.risk.confidence * 100.0
    ));

    if candidate.research.matched_pages > 0 {
        parts.push(format!(
            "research matched {}/{} reviewed page(s), with {} positive signal(s) and {} warning(s)",
            candidate.research.matched_pages,
            candidate.research.pages_reviewed,
            candidate.research.positive_mentions,
            candidate.research.warning_mentions
        ));
    } else if candidate.research.pages_reviewed > 0 {
        parts.push(format!(
            "research reviewed {} page(s) but found no candidate-specific match",
            candidate.research.pages_reviewed
        ));
    }
    let football_warnings = candidate.football_context.warning_count();
    if football_warnings > 0 {
        parts.push(format!(
            "{football_warnings} football context warning category/categories were applied"
        ));
    } else if candidate.football_context.matched_pages == 0 {
        parts.push("football context found no candidate-specific source match".to_string());
    }
    if candidate.learning.confidence_adjustment != 0.0 {
        parts.push(format!(
            "history adjusted confidence by {:+.2} pp",
            candidate.learning.confidence_adjustment * 100.0
        ));
    } else if !candidate.learning.notes.is_empty() {
        parts.push(candidate.learning.notes[0].clone());
    }

    parts.join("; ")
}

fn price_comparison(norsk_tipping_odds: f64, reference_odds: f64) -> String {
    let percent_difference = ((norsk_tipping_odds / reference_odds) - 1.0) * 100.0;
    if percent_difference > 0.0 {
        format!("{percent_difference:.2}% higher than the reference market")
    } else if percent_difference < 0.0 {
        format!(
            "{:.2}% lower than the reference market",
            percent_difference.abs()
        )
    } else {
        "equal to the reference market".to_string()
    }
}

fn clean_zero(value: f64) -> f64 {
    if value.abs() < 0.005 { 0.0 } else { value }
}
