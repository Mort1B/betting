use crate::domain::{BetCandidate, BettingRules};

#[derive(Debug, Clone, Copy)]
pub struct OddsScreeningAgent;

impl OddsScreeningAgent {
    pub fn screen_by_date(
        &self,
        candidates: Vec<BetCandidate>,
        rules: &BettingRules,
    ) -> Vec<BetCandidate> {
        let Some(date) = &rules.date else {
            return candidates;
        };

        candidates
            .into_iter()
            .filter(|candidate| candidate.starts_at.starts_with(date))
            .collect()
    }

    pub fn screen_by_odds(&self, candidate: &BetCandidate, rules: &BettingRules) -> Vec<String> {
        let mut rejections = Vec::new();
        if candidate.norsk_tipping_odds < rules.min_odds {
            rejections.push(format!(
                "Norsk Tipping odds {:.2} are below the {:.2} floor",
                candidate.norsk_tipping_odds, rules.min_odds
            ));
        }
        if candidate.norsk_tipping_odds > rules.max_odds {
            rejections.push(format!(
                "Norsk Tipping odds {:.2} are above the {:.2} ceiling",
                candidate.norsk_tipping_odds, rules.max_odds
            ));
        }
        rejections
    }
}
