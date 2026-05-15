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
                0.30
            }
        });
        let mut flags = Vec::new();
        let mut notes = Vec::new();

        if !candidate.has_independent_probability_signal() {
            flags.push("no independent probability input".to_string());
            confidence -= 0.25;
        }

        let notes_lower = candidate.notes.to_lowercase();
        for (needle, penalty) in [
            ("injury", 0.08),
            ("rotation", 0.07),
            ("lineup unknown", 0.07),
            ("weather", 0.04),
            ("derby", 0.04),
            ("cup", 0.03),
            ("back-to-back", 0.04),
        ] {
            if notes_lower.contains(needle) {
                flags.push(format!("risk note: {needle}"));
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
