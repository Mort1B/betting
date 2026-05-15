use crate::domain::{BetCandidate, ProbabilityAssessment};

#[derive(Debug, Clone, Copy)]
pub struct ProbabilityModelAgent;

impl ProbabilityModelAgent {
    pub fn assess(&self, candidate: &BetCandidate) -> ProbabilityAssessment {
        let implied_probability = candidate.implied_probability();
        let reference_probability = candidate.reference_probability();
        let mut notes = Vec::new();
        let mut sources = Vec::new();

        let estimated_probability = match (candidate.model_probability, reference_probability) {
            (Some(model), Some(reference)) => {
                sources.push("model_probability".to_string());
                sources.push("reference_odds".to_string());
                notes.push("blended model probability with external reference price".to_string());
                (model * 0.70) + (reference * 0.30)
            }
            (Some(model), None) => {
                sources.push("model_probability".to_string());
                model
            }
            (None, Some(reference)) => {
                sources.push("reference_odds".to_string());
                notes.push("using reference odds as market-implied probability".to_string());
                reference
            }
            (None, None) => {
                sources.push("norsk_tipping_implied".to_string());
                notes.push(
                    "no independent probability signal supplied; value cannot be trusted"
                        .to_string(),
                );
                implied_probability
            }
        };

        ProbabilityAssessment {
            estimated_probability,
            implied_probability,
            sources,
            notes,
        }
    }
}
