mod filter;
mod probability;
mod risk;
mod selector;
mod value;

use crate::domain::{BetCandidate, BettingRules, EvaluatedCandidate, RecommendationDecision};
use crate::research::{ResearchDigest, assess_candidate_research};

use filter::OddsScreeningAgent;
use probability::ProbabilityModelAgent;
use risk::RiskAgent;
use selector::DailySelectionAgent;
use value::ValueAgent;

#[derive(Debug, Clone)]
pub struct DailyBetOrchestrator {
    rules: BettingRules,
    odds_screening_agent: OddsScreeningAgent,
    probability_model_agent: ProbabilityModelAgent,
    value_agent: ValueAgent,
    risk_agent: RiskAgent,
    daily_selection_agent: DailySelectionAgent,
}

impl DailyBetOrchestrator {
    pub fn new(rules: BettingRules) -> Self {
        Self {
            rules,
            odds_screening_agent: OddsScreeningAgent,
            probability_model_agent: ProbabilityModelAgent,
            value_agent: ValueAgent,
            risk_agent: RiskAgent,
            daily_selection_agent: DailySelectionAgent,
        }
    }

    pub fn recommend(
        &self,
        candidates: Vec<BetCandidate>,
        research_digest: Option<&ResearchDigest>,
    ) -> RecommendationDecision {
        let candidates = self
            .odds_screening_agent
            .screen_by_date(candidates, &self.rules);
        let mut evaluated = Vec::new();

        for candidate in candidates {
            let mut rejection_reasons = self
                .odds_screening_agent
                .screen_by_odds(&candidate, &self.rules);
            let probability = self.probability_model_agent.assess(&candidate);
            let value = self.value_agent.assess(&candidate, &probability);
            let research = assess_candidate_research(&candidate, research_digest);
            let mut risk = self.risk_agent.assess(&candidate, &probability);
            let research_adjustment = research.confidence_adjustment();
            if research_adjustment != 0.0 {
                risk.confidence = (risk.confidence + research_adjustment).clamp(0.0, 1.0);
                risk.notes.push(format!(
                    "market research confidence adjustment: {:+.2} pp",
                    research_adjustment * 100.0
                ));
            }
            if research.warning_mentions > 0 {
                risk.flags.push(format!(
                    "{} research warning mention(s)",
                    research.warning_mentions
                ));
            }

            rejection_reasons.extend(self.daily_selection_agent.rejections(
                &candidate,
                &probability,
                &value,
                &risk,
                &self.rules,
            ));
            let score = self
                .daily_selection_agent
                .score(&candidate, &probability, &value, &risk);

            evaluated.push(EvaluatedCandidate {
                candidate,
                probability,
                value,
                risk,
                research,
                score,
                rejection_reasons,
            });
        }

        self.daily_selection_agent.choose(evaluated, &self.rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        id: &str,
        odds: f64,
        probability: Option<f64>,
        confidence: Option<f64>,
    ) -> BetCandidate {
        BetCandidate {
            id: id.to_string(),
            sport: "Football".to_string(),
            competition: "Eliteserien".to_string(),
            event: "Rosenborg - Brann".to_string(),
            market: "Double chance".to_string(),
            selection: "Rosenborg or draw".to_string(),
            norsk_tipping_odds: odds,
            model_probability: probability,
            reference_odds: None,
            confidence,
            starts_at: "2026-05-15T18:00:00+02:00".to_string(),
            notes: "lineups stable".to_string(),
        }
    }

    #[test]
    fn recommends_best_candidate_inside_daily_rules() {
        let rules = BettingRules {
            date: Some("2026-05-15".to_string()),
            ..BettingRules::default()
        };
        let recommendation = DailyBetOrchestrator::new(rules).recommend(
            vec![
                candidate("weak", 1.22, Some(0.805), Some(0.72)),
                candidate("best", 1.27, Some(0.835), Some(0.78)),
                candidate("outside", 1.34, Some(0.86), Some(0.80)),
            ],
            None,
        );

        match recommendation {
            RecommendationDecision::Bet { selected, .. } => {
                assert_eq!(selected.candidate.id, "best");
                assert!(selected.is_bettable());
            }
            RecommendationDecision::NoBet { reason, .. } => panic!("expected bet, got {reason}"),
        }
    }

    #[test]
    fn rejects_candidates_without_independent_signal() {
        let recommendation = DailyBetOrchestrator::new(BettingRules::default())
            .recommend(vec![candidate("unsupported", 1.21, None, Some(0.85))], None);

        match recommendation {
            RecommendationDecision::NoBet { reviewed, .. } => {
                assert!(
                    reviewed[0]
                        .rejection_reasons
                        .contains(&"missing independent probability signal".to_string())
                );
            }
            RecommendationDecision::Bet { .. } => {
                panic!("unsupported candidate should not be selected")
            }
        }
    }
}
