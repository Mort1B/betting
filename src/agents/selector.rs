use crate::domain::{
    BetCandidate, BettingRules, EvaluatedCandidate, ProbabilityAssessment, RecommendationDecision,
    RiskAssessment, ValueAssessment,
};

#[derive(Debug, Clone, Copy)]
pub struct DailySelectionAgent;

impl DailySelectionAgent {
    pub fn rejections(
        &self,
        candidate: &BetCandidate,
        probability: &ProbabilityAssessment,
        value: &ValueAssessment,
        risk: &RiskAssessment,
        rules: &BettingRules,
    ) -> Vec<String> {
        let mut rejections = Vec::new();

        if !candidate.has_independent_probability_signal() {
            rejections.push("missing independent probability signal".to_string());
        }
        if probability.estimated_probability < rules.min_estimated_probability {
            rejections.push(format!(
                "estimated probability {:.2}% is below {:.2}% floor",
                probability.estimated_probability * 100.0,
                rules.min_estimated_probability * 100.0
            ));
        }
        if value.edge < rules.min_edge {
            rejections.push(format!(
                "edge {:.2} pp is below {:.2} pp floor",
                value.edge * 100.0,
                rules.min_edge * 100.0
            ));
        }
        if value.expected_value < rules.min_expected_value {
            rejections.push(format!(
                "expected value {:.2}% is below {:.2}% floor",
                value.expected_value * 100.0,
                rules.min_expected_value * 100.0
            ));
        }
        if risk.confidence < rules.min_confidence {
            rejections.push(format!(
                "confidence {:.2}% is below {:.2}% floor",
                risk.confidence * 100.0,
                rules.min_confidence * 100.0
            ));
        }

        rejections
    }

    pub fn score(
        &self,
        candidate: &BetCandidate,
        probability: &ProbabilityAssessment,
        value: &ValueAssessment,
        risk: &RiskAssessment,
    ) -> f64 {
        let probability_score = probability.estimated_probability;
        let edge_score = (value.edge / 0.06).clamp(0.0, 1.0);
        let ev_score = (value.expected_value / 0.08).clamp(0.0, 1.0);
        let odds_band_fit = odds_band_fit(candidate.norsk_tipping_odds);

        100.0
            * ((0.42 * probability_score)
                + (0.26 * edge_score)
                + (0.18 * ev_score)
                + (0.10 * risk.confidence)
                + (0.04 * odds_band_fit))
    }

    pub fn choose(
        &self,
        mut evaluated: Vec<EvaluatedCandidate>,
        rules: &BettingRules,
    ) -> RecommendationDecision {
        evaluated.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let selected_index = evaluated.iter().position(EvaluatedCandidate::is_bettable);
        if let Some(index) = selected_index {
            let selected = evaluated.remove(index);
            let alternatives = evaluated
                .into_iter()
                .filter(EvaluatedCandidate::is_bettable)
                .take(2)
                .collect();
            return RecommendationDecision::Bet {
                selected: Box::new(selected),
                alternatives,
            };
        }

        let reason = if let Some(date) = &rules.date {
            format!("no candidate on {date} passed odds, value, probability, and confidence gates")
        } else {
            "no candidate passed odds, value, probability, and confidence gates".to_string()
        };

        RecommendationDecision::NoBet {
            reason,
            reviewed: evaluated.into_iter().take(6).collect(),
        }
    }
}

fn odds_band_fit(odds: f64) -> f64 {
    let midpoint = (1.15 + 1.30) / 2.0;
    let distance = (odds - midpoint).abs();
    (1.0 - (distance / 0.075)).clamp(0.0, 1.0)
}
