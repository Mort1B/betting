use crate::domain::{BetCandidate, ProbabilityAssessment, ValueAssessment};

#[derive(Debug, Clone, Copy)]
pub struct ValueAgent;

impl ValueAgent {
    pub fn assess(
        &self,
        candidate: &BetCandidate,
        probability: &ProbabilityAssessment,
    ) -> ValueAssessment {
        let expected_value =
            (probability.estimated_probability * candidate.norsk_tipping_odds) - 1.0;
        let edge = probability.estimated_probability - probability.implied_probability;
        let mut value_notes = Vec::new();

        if edge > 0.0 {
            value_notes.push(format!(
                "estimated probability beats Norsk Tipping implied probability by {:.2} pp",
                edge * 100.0
            ));
        } else {
            value_notes.push(format!(
                "estimated probability is {:.2} pp below Norsk Tipping implied probability",
                edge.abs() * 100.0
            ));
        }

        ValueAssessment {
            expected_value,
            edge,
            value_notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_expected_value_and_edge() {
        let candidate = BetCandidate {
            id: "x".to_string(),
            sport: "Football".to_string(),
            competition: "Test".to_string(),
            event: "A - B".to_string(),
            market: "Winner".to_string(),
            selection: "A".to_string(),
            norsk_tipping_odds: 1.28,
            model_probability: Some(0.82),
            reference_odds: None,
            confidence: Some(0.70),
            starts_at: "2026-05-15T18:00:00+02:00".to_string(),
            notes: String::new(),
        };
        let probability = ProbabilityAssessment {
            estimated_probability: 0.82,
            implied_probability: candidate.implied_probability(),
            sources: vec!["model_probability".to_string()],
            notes: vec![],
        };

        let value = ValueAgent.assess(&candidate, &probability);

        assert!((value.expected_value - 0.0496).abs() < 0.0001);
        assert!((value.edge - 0.03875).abs() < 0.0001);
    }
}
