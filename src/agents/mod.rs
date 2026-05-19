mod filter;
mod learning;
mod probability;
mod risk;
mod selector;
mod value;

use crate::domain::{BetCandidate, BettingRules, EvaluatedCandidate, RecommendationDecision};
use crate::football_context::assess_football_context;
use crate::research::{ResearchDigest, assess_candidate_research};

use filter::OddsScreeningAgent;
use learning::LearningAgent;
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
    learning_agent: LearningAgent,
    daily_selection_agent: DailySelectionAgent,
}

impl DailyBetOrchestrator {
    #[cfg(test)]
    pub fn new(rules: BettingRules) -> Self {
        Self::with_learning_agent(rules, LearningAgent::disabled())
    }

    pub fn from_env(rules: BettingRules) -> Result<Self, String> {
        Ok(Self::with_learning_agent(rules, LearningAgent::from_env()?))
    }

    fn with_learning_agent(rules: BettingRules, learning_agent: LearningAgent) -> Self {
        Self {
            rules,
            odds_screening_agent: OddsScreeningAgent,
            probability_model_agent: ProbabilityModelAgent,
            value_agent: ValueAgent,
            risk_agent: RiskAgent,
            learning_agent,
            daily_selection_agent: DailySelectionAgent,
        }
    }

    pub fn recommend(
        &self,
        candidates: Vec<BetCandidate>,
        research_digest: Option<&ResearchDigest>,
    ) -> RecommendationDecision {
        let date_screened = self
            .odds_screening_agent
            .screen_by_date(candidates.clone(), &self.rules);
        let used_date_fallback = self.rules.date.is_some() && date_screened.is_empty();
        let candidates = if used_date_fallback {
            candidates
        } else {
            date_screened
        }
        .into_iter()
        .filter(|candidate| {
            self.rules
                .is_inside_research_odds_band(candidate.norsk_tipping_odds)
        })
        .collect::<Vec<_>>();
        let mut evaluated = Vec::new();

        for candidate in candidates {
            let mut rejection_reasons = self
                .odds_screening_agent
                .screen_by_odds(&candidate, &self.rules);
            if let (true, Some(date)) = (used_date_fallback, &self.rules.date) {
                rejection_reasons.push(format!(
                    "no candidate matched requested date {date}; using best available loaded board"
                ));
            }
            let probability = self.probability_model_agent.assess(&candidate);
            let value = self.value_agent.assess(&candidate, &probability);
            let research = assess_candidate_research(&candidate, research_digest);
            let football_context = assess_football_context(&candidate, research_digest);
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
            if football_context.confidence_adjustment != 0.0 {
                risk.confidence =
                    (risk.confidence + football_context.confidence_adjustment).clamp(0.0, 1.0);
                risk.notes.push(format!(
                    "football context confidence adjustment: {:+.2} pp",
                    football_context.confidence_adjustment * 100.0
                ));
            }
            let football_warning_count = football_context.warning_count();
            if football_warning_count > 0 {
                risk.flags.push(format!(
                    "{football_warning_count} football context warning(s)"
                ));
            }
            let learning = self.learning_agent.assess(&candidate, &football_context);
            if learning.confidence_adjustment != 0.0 {
                risk.confidence =
                    (risk.confidence + learning.confidence_adjustment).clamp(0.0, 1.0);
                risk.notes.push(format!(
                    "learning confidence adjustment: {:+.2} pp",
                    learning.confidence_adjustment * 100.0
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
                football_context,
                learning,
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
                candidate("best", 1.27, Some(0.885), Some(0.80)),
                candidate("solid", 1.18, Some(0.870), Some(0.74)),
                candidate("third", 1.20, Some(0.860), Some(0.72)),
                candidate("fourth", 1.19, Some(0.860), Some(0.72)),
                candidate("fifth", 1.21, Some(0.860), Some(0.72)),
            ],
            None,
        );

        match recommendation {
            RecommendationDecision::Bet {
                selected,
                alternatives,
            } => {
                assert_eq!(selected.candidate.id, "best");
                assert!(selected.is_bettable());
                assert_eq!(alternatives.len(), 4);
            }
            RecommendationDecision::BestAvailable { reason, .. } => {
                panic!("expected strict bet, got fallback candidates: {reason}")
            }
            RecommendationDecision::NoBet { reason, .. } => panic!("expected bet, got {reason}"),
        }
    }

    #[test]
    fn accepts_market_implied_probability_when_confidence_is_strong() {
        let recommendation = DailyBetOrchestrator::new(BettingRules::default())
            .recommend(vec![candidate("unsupported", 1.21, None, Some(0.85))], None);

        match recommendation {
            RecommendationDecision::Bet { selected, .. } => {
                assert_eq!(selected.candidate.id, "unsupported");
                assert!(selected.is_bettable());
                assert!(
                    selected
                        .probability
                        .sources
                        .contains(&"norsk_tipping_market_implied".to_string())
                );
            }
            RecommendationDecision::BestAvailable { reason, .. } => {
                panic!("strong market-implied candidate should be selectable: {reason}")
            }
            RecommendationDecision::NoBet { reason, .. } => {
                panic!("expected candidate, got no bet: {reason}")
            }
        }
    }

    #[test]
    fn returns_no_bet_when_no_viable_candidates_exist() {
        let rules = BettingRules {
            date: Some("2026-05-19".to_string()),
            ..BettingRules::default()
        };
        let recommendation = DailyBetOrchestrator::new(rules).recommend(Vec::new(), None);

        match recommendation {
            RecommendationDecision::NoBet { reason, reviewed } => {
                assert_eq!(reason, "no viable candidates were supplied for 2026-05-19");
                assert!(reviewed.is_empty());
            }
            RecommendationDecision::Bet { .. } => panic!("empty slate should not be a bet"),
            RecommendationDecision::BestAvailable { reason, .. } => {
                panic!("empty slate should not show fallback candidates: {reason}")
            }
        }
    }

    #[test]
    fn fills_top_five_from_best_available_when_date_has_no_matches() {
        let rules = BettingRules {
            date: Some("2026-05-16".to_string()),
            ..BettingRules::default()
        };
        let recommendation = DailyBetOrchestrator::new(rules).recommend(
            vec![
                candidate("one", 1.22, Some(0.805), Some(0.72)),
                candidate("two", 1.27, Some(0.835), Some(0.78)),
                candidate("three", 1.18, Some(0.870), Some(0.74)),
                candidate("four", 1.19, Some(0.860), Some(0.74)),
                candidate("five", 1.20, Some(0.860), Some(0.74)),
                candidate("outside", 1.34, Some(0.900), Some(0.90)),
            ],
            None,
        );

        match recommendation {
            RecommendationDecision::BestAvailable { picks, reason } => {
                assert_eq!(picks.len(), 5);
                assert!(
                    picks
                        .iter()
                        .all(|pick| pick.candidate.norsk_tipping_odds <= 1.30)
                );
                assert!(reason.contains("no candidate passed every strict gate"));
                assert!(picks.iter().all(|pick| {
                    pick.rejection_reasons
                        .iter()
                        .any(|reason| reason.contains("no candidate matched requested date"))
                }));
            }
            RecommendationDecision::Bet { .. } => panic!("date fallback should not be strict bet"),
            RecommendationDecision::NoBet { reason, .. } => {
                panic!("expected top 5 fallback candidates, got no bet: {reason}")
            }
        }
    }

    #[test]
    fn keeps_slack_odds_as_fallback_and_excludes_hard_ceiling() {
        let recommendation = DailyBetOrchestrator::new(BettingRules::default()).recommend(
            vec![
                candidate("strict", 1.22, Some(0.850), Some(0.78)),
                candidate("slack", 1.34, Some(0.850), Some(0.78)),
                candidate("too-high", 1.36, Some(0.900), Some(0.90)),
            ],
            None,
        );

        match recommendation {
            RecommendationDecision::BestAvailable { picks, .. } => {
                assert_eq!(picks.len(), 2);
                assert!(picks.iter().any(|pick| pick.candidate.id == "strict"));
                assert!(picks.iter().any(|pick| {
                    pick.candidate.id == "slack"
                        && pick
                            .rejection_reasons
                            .iter()
                            .any(|reason| reason.contains("slack fallback only"))
                }));
                assert!(!picks.iter().any(|pick| pick.candidate.id == "too-high"));
            }
            RecommendationDecision::Bet { .. } => panic!("slack candidate should force fallback"),
            RecommendationDecision::NoBet { reason, .. } => {
                panic!("expected ranked candidates, got no bet: {reason}")
            }
        }
    }
}
