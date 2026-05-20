use std::fs;

use serde_json::{Value, json};

use crate::domain::{
    BettingRules, EvaluatedCandidate, FootballContextStatus, RecommendationDecision,
};

#[derive(Debug, Clone)]
pub(crate) struct JsonReportMeta {
    pub(crate) final_text_report: String,
    pub(crate) deterministic_text_report: String,
    pub(crate) ai_enabled: bool,
    pub(crate) ai_used: bool,
    pub(crate) ai_fallback_reason: Option<String>,
    pub(crate) reference_provider_notes: Vec<String>,
    pub(crate) football_data_provider_notes: Vec<String>,
}

pub(crate) fn write_json_report(
    path: &str,
    rules: &BettingRules,
    recommendation: &RecommendationDecision,
    meta: &JsonReportMeta,
) -> Result<(), String> {
    let value = json_report_value(rules, recommendation, meta);
    let content = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to encode JSON report: {error}"))?;
    fs::write(path, format!("{content}\n"))
        .map_err(|error| format!("failed to write JSON report {path}: {error}"))
}

fn json_report_value(
    rules: &BettingRules,
    recommendation: &RecommendationDecision,
    meta: &JsonReportMeta,
) -> Value {
    json!({
        "schema_version": 1,
        "report_date": rules.date,
        "sport_scope": rules.sport_scope.display_name(),
        "pick_target": rules.pick_count,
        "odds_rules": {
            "preferred_min": rules.min_odds,
            "preferred_max": rules.max_odds,
            "hard_research_max": rules.max_research_odds,
            "min_estimated_probability": rules.min_estimated_probability,
            "min_confidence": rules.min_confidence,
            "min_edge": rules.min_edge,
            "min_expected_value": rules.min_expected_value
        },
        "ai": {
            "enabled": meta.ai_enabled,
            "used": meta.ai_used,
            "fallback_reason": meta.ai_fallback_reason
        },
        "reference_provider_notes": meta.reference_provider_notes,
        "football_data_provider_notes": meta.football_data_provider_notes,
        "decision": decision_value(recommendation),
        "reports": {
            "final_text": meta.final_text_report,
            "deterministic_text": meta.deterministic_text_report
        }
    })
}

fn decision_value(recommendation: &RecommendationDecision) -> Value {
    match recommendation {
        RecommendationDecision::Bet {
            selected,
            alternatives,
        } => {
            let picks = std::iter::once(selected.as_ref())
                .chain(alternatives.iter())
                .enumerate()
                .map(|(index, candidate)| candidate_value(index + 1, candidate))
                .collect::<Vec<_>>();

            json!({
                "kind": "bet",
                "reason": null,
                "picks": picks
            })
        }
        RecommendationDecision::BestAvailable { reason, picks } => {
            let picks = picks
                .iter()
                .enumerate()
                .map(|(index, candidate)| candidate_value(index + 1, candidate))
                .collect::<Vec<_>>();

            json!({
                "kind": "top_candidates",
                "reason": reason,
                "picks": picks
            })
        }
        RecommendationDecision::NoBet { reason, reviewed } => {
            let reviewed = reviewed
                .iter()
                .enumerate()
                .map(|(index, candidate)| candidate_value(index + 1, candidate))
                .collect::<Vec<_>>();

            json!({
                "kind": "no_bet",
                "reason": reason,
                "picks": [],
                "reviewed": reviewed
            })
        }
    }
}

fn candidate_value(rank: usize, candidate: &EvaluatedCandidate) -> Value {
    json!({
        "rank": rank,
        "strict_rules_status": strict_status(candidate),
        "is_bettable": candidate.is_bettable(),
        "rejection_reasons": candidate.rejection_reasons,
        "candidate": {
            "id": candidate.candidate.id,
            "sport": candidate.candidate.sport,
            "competition": candidate.candidate.competition,
            "event": candidate.candidate.event,
            "market": candidate.candidate.market,
            "selection": candidate.candidate.selection,
            "norsk_tipping_odds": candidate.candidate.norsk_tipping_odds,
            "model_probability": candidate.candidate.model_probability,
            "reference_odds": candidate.candidate.reference_odds,
            "confidence": candidate.candidate.confidence,
            "starts_at": candidate.candidate.starts_at,
            "notes": candidate.candidate.notes
        },
        "probability": {
            "estimated_probability": candidate.probability.estimated_probability,
            "norsk_tipping_implied_probability": candidate.probability.implied_probability,
            "sources": candidate.probability.sources,
            "notes": candidate.probability.notes
        },
        "value": {
            "expected_value": candidate.value.expected_value,
            "edge": candidate.value.edge,
            "notes": candidate.value.value_notes
        },
        "risk": {
            "confidence": candidate.risk.confidence,
            "confidence_score": (candidate.risk.confidence * 100.0).round() as u32,
            "flags": candidate.risk.flags,
            "notes": candidate.risk.notes
        },
        "research": {
            "pages_reviewed": candidate.research.pages_reviewed,
            "matched_pages": candidate.research.matched_pages,
            "positive_mentions": candidate.research.positive_mentions,
            "warning_mentions": candidate.research.warning_mentions,
            "price_hints": candidate.research.price_hints,
            "notes": candidate.research.notes
        },
        "football_context": {
            "matched_pages": candidate.football_context.matched_pages,
            "confidence_adjustment": candidate.football_context.confidence_adjustment,
            "categories": candidate.football_context.categories.iter().map(|category| {
                json!({
                    "name": category.name,
                    "status": context_status(category.status),
                    "evidence": category.evidence
                })
            }).collect::<Vec<_>>(),
            "notes": candidate.football_context.notes
        },
        "learning": {
            "bucket": candidate.learning.bucket,
            "settled_samples": candidate.learning.settled_samples,
            "wins": candidate.learning.wins,
            "losses": candidate.learning.losses,
            "hit_rate": candidate.learning.hit_rate,
            "confidence_adjustment": candidate.learning.confidence_adjustment,
            "notes": candidate.learning.notes
        },
        "score": candidate.score
    })
}

fn strict_status(candidate: &EvaluatedCandidate) -> &'static str {
    if candidate.is_bettable() {
        "pass"
    } else {
        "fallback"
    }
}

fn context_status(status: FootballContextStatus) -> &'static str {
    match status {
        FootballContextStatus::Positive => "positive",
        FootballContextStatus::Warning => "warning",
        FootballContextStatus::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        BetCandidate, FootballContextAssessment, FootballContextCategory, LearningAssessment,
        ProbabilityAssessment, ResearchAssessment, RiskAssessment, SportScope, ValueAssessment,
    };

    #[test]
    fn renders_complete_json_report_with_text_and_ranked_picks() {
        let rules = BettingRules {
            date: Some("2026-05-20".to_string()),
            sport_scope: SportScope::Football,
            ..BettingRules::default()
        };
        let recommendation = RecommendationDecision::BestAvailable {
            reason: "showing best available".to_string(),
            picks: vec![evaluated_candidate()],
        };
        let meta = JsonReportMeta {
            final_text_report: "final report".to_string(),
            deterministic_text_report: "deterministic report".to_string(),
            ai_enabled: true,
            ai_used: false,
            ai_fallback_reason: Some("truncated".to_string()),
            reference_provider_notes: vec!["provider summary".to_string()],
            football_data_provider_notes: vec!["context summary".to_string()],
        };

        let value = json_report_value(&rules, &recommendation, &meta);

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["reports"]["final_text"], "final report");
        assert_eq!(value["ai"]["fallback_reason"], "truncated");
        assert_eq!(value["football_data_provider_notes"][0], "context summary");
        assert_eq!(value["decision"]["picks"][0]["rank"], 1);
        assert_eq!(
            value["decision"]["picks"][0]["candidate"]["event"],
            "Rosenborg - Brann"
        );
    }

    fn evaluated_candidate() -> EvaluatedCandidate {
        EvaluatedCandidate {
            candidate: BetCandidate {
                id: "pick-1".to_string(),
                sport: "Football".to_string(),
                competition: "Eliteserien".to_string(),
                event: "Rosenborg - Brann".to_string(),
                market: "Match winner".to_string(),
                selection: "Rosenborg".to_string(),
                norsk_tipping_odds: 1.22,
                model_probability: Some(0.84),
                reference_odds: None,
                confidence: Some(0.75),
                starts_at: "2026-05-20T18:00:00+02:00".to_string(),
                notes: "note".to_string(),
            },
            probability: ProbabilityAssessment {
                estimated_probability: 0.84,
                implied_probability: 1.0 / 1.22,
                sources: vec!["model_probability".to_string()],
                notes: Vec::new(),
            },
            value: ValueAssessment {
                expected_value: 0.02,
                edge: 0.03,
                value_notes: Vec::new(),
            },
            risk: RiskAssessment {
                confidence: 0.75,
                flags: Vec::new(),
                notes: Vec::new(),
            },
            research: ResearchAssessment::empty(),
            football_context: FootballContextAssessment {
                matched_pages: 0,
                categories: vec![FootballContextCategory {
                    name: "Form".to_string(),
                    status: FootballContextStatus::Unknown,
                    evidence: Vec::new(),
                }],
                confidence_adjustment: 0.0,
                notes: Vec::new(),
            },
            learning: LearningAssessment::no_history(),
            score: 77.0,
            rejection_reasons: Vec::new(),
        }
    }
}
