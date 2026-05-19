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

        if probability.estimated_probability < rules.min_estimated_probability {
            rejections.push(format!(
                "estimated probability {:.2}% is below {:.2}% floor",
                probability.estimated_probability * 100.0,
                rules.min_estimated_probability * 100.0
            ));
        }
        if candidate.has_independent_probability_signal() && value.edge < rules.min_edge {
            rejections.push(format!(
                "edge {:.2} pp is below {:.2} pp floor",
                value.edge * 100.0,
                rules.min_edge * 100.0
            ));
        }
        if candidate.has_independent_probability_signal()
            && value.expected_value < rules.min_expected_value
        {
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
        let evidence_score = if candidate.has_independent_probability_signal() {
            ((edge_score + ev_score) / 2.0).max(risk.confidence)
        } else {
            risk.confidence
        };

        100.0
            * ((0.55 * probability_score)
                + (0.25 * risk.confidence)
                + (0.10 * evidence_score)
                + (0.06 * edge_score)
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

        if evaluated.is_empty() {
            let reason = if let Some(date) = &rules.date {
                format!("no candidates were supplied for {date}")
            } else {
                "no candidates were supplied".to_string()
            };
            return RecommendationDecision::NoBet {
                reason,
                reviewed: Vec::new(),
            };
        }

        let mut picks = Vec::new();
        for candidate in evaluated.iter().filter(|candidate| candidate.is_bettable()) {
            if picks.len() == rules.pick_count {
                break;
            }
            picks.push(candidate.clone());
        }
        self.fill_from_best_available(&mut picks, &evaluated, rules, true);
        self.fill_from_best_available(&mut picks, &evaluated, rules, false);

        if picks.is_empty() {
            let reason = if let Some(date) = &rules.date {
                format!("no candidates were supplied for {date}")
            } else {
                "no candidates were supplied".to_string()
            };
            return RecommendationDecision::NoBet {
                reason,
                reviewed: Vec::new(),
            };
        }

        if picks.iter().all(EvaluatedCandidate::is_bettable) {
            let selected = picks.remove(0);
            return RecommendationDecision::Bet {
                selected: Box::new(selected),
                alternatives: picks,
            };
        }

        let strict_count = picks
            .iter()
            .filter(|candidate| candidate.is_bettable())
            .count();
        let reason = if strict_count == 0 {
            "no candidate passed every strict gate; showing the best available ranked candidates"
                .to_string()
        } else if picks.len() < rules.pick_count {
            format!(
                "only {strict_count} candidate(s) passed every strict gate and only {} {} candidate(s) were available; showing all ranked candidates for the top {} target",
                picks.len(),
                rules.sport_scope.display_name(),
                rules.pick_count
            )
        } else {
            format!(
                "only {strict_count} candidate(s) passed every strict gate; filling the top {} with best available fallbacks",
                rules.pick_count
            )
        };

        RecommendationDecision::BestAvailable { reason, picks }
    }

    fn fill_from_best_available(
        &self,
        picks: &mut Vec<EvaluatedCandidate>,
        evaluated: &[EvaluatedCandidate],
        rules: &BettingRules,
        require_odds_band: bool,
    ) {
        for candidate in evaluated {
            if picks.len() == rules.pick_count {
                break;
            }
            if require_odds_band
                && !is_inside_requested_odds_band(candidate.candidate.norsk_tipping_odds, rules)
            {
                continue;
            }
            if !picks
                .iter()
                .any(|picked: &EvaluatedCandidate| picked.candidate.id == candidate.candidate.id)
            {
                picks.push(candidate.clone());
            }
        }
    }
}

fn is_inside_requested_odds_band(odds: f64, rules: &BettingRules) -> bool {
    rules.is_inside_preferred_odds_band(odds)
}

fn odds_band_fit(odds: f64) -> f64 {
    let midpoint = (1.10 + 1.30) / 2.0;
    let distance = (odds - midpoint).abs();
    (1.0 - (distance / 0.10)).clamp(0.0, 1.0)
}
