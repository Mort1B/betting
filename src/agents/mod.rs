mod filter;
mod learning;
mod probability;
mod risk;
mod selector;
#[cfg(test)]
mod tests;
mod value;

use crate::domain::{
    BetCandidate, BettingRules, EvaluatedCandidate, FootballContextAssessment,
    FootballContextStatus, ProbabilityAssessment, RecommendationDecision, RiskAssessment,
};
use crate::football_context::assess_football_context;
use crate::research::{ResearchDigest, assess_candidate_research};

use filter::OddsScreeningAgent;
pub(crate) use learning::LearningAgent;
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
        Self::with_learning_agent(rules, LearningAgent::from_entries(Vec::new()))
    }

    pub(crate) fn with_learning_agent(rules: BettingRules, learning_agent: LearningAgent) -> Self {
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
            apply_missing_context_risk(&probability, &football_context, &mut risk);
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

fn apply_missing_context_risk(
    probability: &ProbabilityAssessment,
    football_context: &FootballContextAssessment,
    risk: &mut RiskAssessment,
) {
    if !all_context_unknown(football_context) {
        return;
    }

    risk.flags
        .push("missing football context evidence".to_string());
    risk.notes
        .push("all football context checklist items are unknown".to_string());
    risk.confidence = (risk.confidence - 0.08).clamp(0.0, 1.0);

    if uses_norsk_tipping_implied_only(probability) {
        risk.flags
            .push("market-implied probability lacks independent or context evidence".to_string());
        risk.notes.push(
            "estimated probability equals Norsk Tipping implied probability; strict recommendation requires more context"
                .to_string(),
        );
        risk.confidence = (risk.confidence - 0.15).clamp(0.0, 1.0);
    }
}

fn all_context_unknown(football_context: &FootballContextAssessment) -> bool {
    football_context
        .categories
        .iter()
        .all(|category| category.status == FootballContextStatus::Unknown)
}

fn uses_norsk_tipping_implied_only(probability: &ProbabilityAssessment) -> bool {
    probability
        .sources
        .iter()
        .any(|source| source == "norsk_tipping_market_implied")
        && (probability.estimated_probability - probability.implied_probability).abs() < 0.0001
}
