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
            .filter(|candidate| starts_inside_date_window(&candidate.starts_at, date, rules))
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
        if rules.is_inside_slack_odds_band(candidate.norsk_tipping_odds) {
            rejections.push(format!(
                "Norsk Tipping odds {:.2} are above preferred ceiling {:.2}; slack fallback only",
                candidate.norsk_tipping_odds, rules.max_odds
            ));
        } else if candidate.norsk_tipping_odds > rules.max_research_odds {
            rejections.push(format!(
                "Norsk Tipping odds {:.2} are above research ceiling {:.2}",
                candidate.norsk_tipping_odds, rules.max_research_odds
            ));
        }
        rejections
    }
}

fn starts_inside_date_window(starts_at: &str, date: &str, rules: &BettingRules) -> bool {
    if let Some(latest_start) = rules.latest_start.as_deref() {
        return starts_at
            .get(..latest_start.len())
            .is_some_and(|prefix| prefix <= latest_start && prefix >= date);
    }
    starts_at.starts_with(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_screen_keeps_next_morning_inside_latest_start_window() {
        let rules = BettingRules {
            date: Some("2026-05-16".to_string()),
            latest_start: Some("2026-05-17T05:00".to_string()),
            ..BettingRules::default()
        };
        let screened = OddsScreeningAgent.screen_by_date(
            vec![
                candidate("same-day", "2026-05-16T18:00:00.000+02:00"),
                candidate("next-morning", "2026-05-17T04:30:00.000+02:00"),
                candidate("too-late", "2026-05-17T05:30:00.000+02:00"),
            ],
            &rules,
        );

        assert_eq!(
            screened
                .iter()
                .map(|candidate| candidate.id.as_str())
                .collect::<Vec<_>>(),
            vec!["same-day", "next-morning"]
        );
    }

    fn candidate(id: &str, starts_at: &str) -> BetCandidate {
        BetCandidate {
            id: id.to_string(),
            sport: "Football".to_string(),
            competition: "League".to_string(),
            event: "Home - Away".to_string(),
            market: "Main market".to_string(),
            selection: "Home".to_string(),
            norsk_tipping_odds: 1.22,
            model_probability: None,
            reference_odds: None,
            confidence: None,
            starts_at: starts_at.to_string(),
            notes: String::new(),
        }
    }
}
