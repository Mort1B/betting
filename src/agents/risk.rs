use crate::domain::{BetCandidate, ProbabilityAssessment, RiskAssessment};

#[derive(Debug, Clone, Copy)]
pub struct RiskAgent;

impl RiskAgent {
    pub fn assess(
        &self,
        candidate: &BetCandidate,
        probability: &ProbabilityAssessment,
    ) -> RiskAssessment {
        let mut confidence = candidate.confidence.unwrap_or_else(|| {
            if candidate.has_independent_probability_signal() {
                0.58
            } else {
                candidate.implied_probability().clamp(0.55, 0.82)
            }
        });
        let mut flags = Vec::new();
        let mut notes = Vec::new();

        if !candidate.has_independent_probability_signal() {
            notes.push(
                "probability baseline is market-implied, not an independent value model"
                    .to_string(),
            );
        }

        let context = format!(
            "{} {} {} {} {} {}",
            candidate.sport,
            candidate.competition,
            candidate.event,
            candidate.market,
            candidate.selection,
            candidate.notes
        )
        .to_lowercase();
        for (needle, penalty) in [
            ("injury", 0.08),
            ("rotation", 0.07),
            ("lineup unknown", 0.07),
            ("weather", 0.04),
            ("derby", 0.04),
            ("cup", 0.03),
            ("back-to-back", 0.04),
            ("friendly", 0.05),
            ("privatlandskamp", 0.05),
            ("underholdning", 0.12),
            ("eurovision", 0.12),
            ("outright", 0.10),
            ("future", 0.08),
            ("top 3", 0.08),
            ("top 5", 0.07),
            ("top 10", 0.06),
            ("topp 3", 0.08),
            ("topp 5", 0.07),
            ("topp 10", 0.06),
            ("kun singelspill", 0.04),
        ] {
            if context.contains(needle) {
                flags.push(format!("context risk: {needle}"));
                confidence -= penalty;
            }
        }

        if probability.estimated_probability < probability.implied_probability {
            flags.push("negative probability edge".to_string());
            confidence -= 0.08;
        }

        confidence = confidence.clamp(0.0, 1.0);
        if flags.is_empty() {
            notes.push("no material risk flags in candidate notes".to_string());
        } else {
            notes.push(format!("{} risk flag(s) applied", flags.len()));
        }

        RiskAssessment {
            confidence,
            flags,
            notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(sport: &str, competition: &str, market: &str, notes: &str) -> BetCandidate {
        BetCandidate {
            id: "c1".to_string(),
            sport: sport.to_string(),
            competition: competition.to_string(),
            event: "Event".to_string(),
            market: market.to_string(),
            selection: "Selection".to_string(),
            norsk_tipping_odds: 1.20,
            model_probability: None,
            reference_odds: None,
            confidence: Some(0.80),
            starts_at: "2026-05-16T18:00:00+02:00".to_string(),
            notes: notes.to_string(),
        }
    }

    #[test]
    fn keeps_market_implied_candidates_viable_without_external_odds() {
        let candidate = candidate("Football", "Eliteserien", "Main market", "lineups stable");
        let probability = ProbabilityAssessment {
            estimated_probability: candidate.implied_probability(),
            implied_probability: candidate.implied_probability(),
            sources: vec!["norsk_tipping_market_implied".to_string()],
            notes: Vec::new(),
        };

        let risk = RiskAgent.assess(&candidate, &probability);

        assert!(risk.confidence >= 0.65);
        assert!(risk.flags.is_empty());
        assert!(
            risk.notes
                .iter()
                .any(|note| note.contains("market-implied"))
        );
    }

    #[test]
    fn penalizes_entertainment_and_special_markets() {
        let candidate = candidate(
            "Underholdning",
            "Eurovision Song Contest",
            "Kommer topp 10 (Kun singelspill)",
            "",
        );
        let probability = ProbabilityAssessment {
            estimated_probability: candidate.implied_probability(),
            implied_probability: candidate.implied_probability(),
            sources: vec!["norsk_tipping_market_implied".to_string()],
            notes: Vec::new(),
        };

        let risk = RiskAgent.assess(&candidate, &probability);

        assert!(risk.confidence < 0.65);
        assert!(risk.flags.iter().any(|flag| flag.contains("eurovision")));
        assert!(risk.flags.iter().any(|flag| flag.contains("topp 10")));
    }
}
