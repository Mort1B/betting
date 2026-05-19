use std::env;

use crate::domain::{BettingRules, EvaluatedCandidate, RecommendationDecision};

mod details;

use details::push_ranked_candidate_details;

pub fn render_recommendation(
    rules: &BettingRules,
    recommendation: &RecommendationDecision,
) -> String {
    let mut output = String::new();
    output.push_str("Daily betting agent recommendation\n");
    output.push_str("==================================\n\n");
    output.push_str(&format!(
        "Rules: Norsk Tipping preferred odds {:.2}-{:.2}, hard research ceiling {:.2}, min probability {:.2}%, min confidence {:.2}%, min edge {:.2} pp when model/reference data exists\n",
        rules.min_odds,
        rules.max_odds,
        rules.max_research_odds,
        rules.min_estimated_probability * 100.0,
        rules.min_confidence * 100.0,
        rules.min_edge * 100.0
    ));
    if let Some(date) = &rules.date {
        output.push_str(&format!("Date filter: {date}\n"));
    }
    push_run_summary(&mut output, rules, recommendation);
    output.push('\n');

    match recommendation {
        RecommendationDecision::Bet {
            selected,
            alternatives,
        } => {
            output.push_str("Decision: BET\n\n");
            output.push_str(&format!("Top {} candidates:\n\n", 1 + alternatives.len()));
            push_ranked_candidate_details(&mut output, 1, selected);
            for (index, alternative) in alternatives.iter().enumerate() {
                output.push('\n');
                push_ranked_candidate_details(&mut output, index + 2, alternative);
            }
        }
        RecommendationDecision::BestAvailable { reason, picks } => {
            output.push_str(&format!("Decision: TOP {} CANDIDATES\n", rules.pick_count));
            output.push_str(&format!("Reason: {reason}\n\n"));
            output.push_str("These are ranked best available candidates, not guaranteed picks. Check the strict rules status before placing a bet.\n\n");
            output.push_str(&format!("Top {} candidates:\n\n", picks.len()));
            for (index, candidate) in picks.iter().enumerate() {
                if index > 0 {
                    output.push('\n');
                }
                push_ranked_candidate_details(&mut output, index + 1, candidate);
            }
        }
        RecommendationDecision::NoBet { reason, reviewed } => {
            output.push_str("Decision: NO BET\n");
            output.push_str(&format!("Reason: {reason}\n\n"));
            if reviewed.is_empty() {
                output
                    .push_str("No viable bets available; no candidates were available to rank.\n");
            } else {
                output.push_str("Closest reviewed candidates:\n");
                for candidate in reviewed {
                    output.push_str(&format!(
                        "- {} | {} | {} @ {:.2} | score {:.1} | research {}/{} warn {} | rejected: {}\n",
                        candidate.candidate.event,
                        candidate.candidate.market,
                        candidate.candidate.selection,
                        candidate.candidate.norsk_tipping_odds,
                        candidate.score,
                        candidate.research.matched_pages,
                        candidate.research.pages_reviewed,
                        candidate.research.warning_mentions,
                        candidate.rejection_reasons.join("; ")
                    ));
                }
            }
        }
    }

    output
}

fn push_run_summary(
    output: &mut String,
    rules: &BettingRules,
    recommendation: &RecommendationDecision,
) {
    let candidates = ranked_candidates(recommendation);
    output.push_str(&format!(
        "Scope: {} | Pick target: {}\n",
        rules.sport_scope.display_name(),
        rules.pick_count
    ));
    output.push_str(&format!("Pick history: {}\n", history_status()));
    if candidates.is_empty() {
        output.push_str("Source coverage: no ranked candidates\n");
        output.push_str("Learning summary: no ranked candidates\n");
        return;
    }

    output.push_str(&format!(
        "Source coverage: {}\n",
        source_coverage(&candidates)
    ));
    output.push_str(&format!(
        "Missing context: {}\n",
        missing_context(&candidates)
    ));
    output.push_str(&format!(
        "Learning summary: {}\n",
        learning_summary(&candidates)
    ));
}

fn ranked_candidates(recommendation: &RecommendationDecision) -> Vec<&EvaluatedCandidate> {
    match recommendation {
        RecommendationDecision::Bet {
            selected,
            alternatives,
        } => std::iter::once(selected.as_ref())
            .chain(alternatives.iter())
            .collect(),
        RecommendationDecision::BestAvailable { picks, .. } => picks.iter().collect(),
        RecommendationDecision::NoBet { reviewed, .. } => reviewed.iter().collect(),
    }
}

fn history_status() -> String {
    if env::var("BETTING_HISTORY_OUTPUT").is_ok() {
        "enabled; writes history.jsonl".to_string()
    } else if env::var("BETTING_HISTORY_INPUT").is_ok() {
        "read-only history input".to_string()
    } else {
        "disabled for this run".to_string()
    }
}

fn source_coverage(candidates: &[&EvaluatedCandidate]) -> String {
    let pages_reviewed = candidates
        .iter()
        .map(|candidate| candidate.research.pages_reviewed)
        .max()
        .unwrap_or(0);
    let matched_candidates = candidates
        .iter()
        .filter(|candidate| candidate.research.matched_pages > 0)
        .count();
    let warning_candidates = candidates
        .iter()
        .filter(|candidate| candidate.research.warning_mentions > 0)
        .count();
    let source_errors = candidates
        .iter()
        .flat_map(|candidate| candidate.research.notes.iter())
        .filter(|note| note.starts_with("source error:"))
        .count();

    format!(
        "reviewed up to {pages_reviewed} page(s); matched {matched_candidates}/{} pick(s); warnings on {warning_candidates}; source errors {source_errors}",
        candidates.len()
    )
}

fn missing_context(candidates: &[&EvaluatedCandidate]) -> String {
    let unknown = candidates
        .iter()
        .flat_map(|candidate| candidate.football_context.categories.iter())
        .filter(|category| category.status.label() == "unknown")
        .count();
    let warnings = candidates
        .iter()
        .map(|candidate| candidate.football_context.warning_count())
        .sum::<usize>();

    format!("{unknown} unknown checklist item(s); {warnings} warning category/categories")
}

fn learning_summary(candidates: &[&EvaluatedCandidate]) -> String {
    let adjusted = candidates
        .iter()
        .filter(|candidate| candidate.learning.confidence_adjustment != 0.0)
        .count();
    if adjusted > 0 {
        return format!("adjusted {adjusted}/{} pick(s)", candidates.len());
    }
    candidates
        .first()
        .and_then(|candidate| candidate.learning.notes.first())
        .cloned()
        .unwrap_or_else(|| "no learning note available".to_string())
}
