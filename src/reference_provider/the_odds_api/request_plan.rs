use super::http::CreditUsage;

#[derive(Debug, Default)]
pub(super) struct FetchStats {
    pub(super) sport_odds_requests: usize,
    pub(super) sport_odds_successes: usize,
    pub(super) event_list_requests: usize,
    pub(super) event_list_successes: usize,
    pub(super) event_odds_requests: usize,
    pub(super) event_odds_successes: usize,
    pub(super) credit_remaining: Option<String>,
    pub(super) credit_used: Option<String>,
    pub(super) credit_last: Option<String>,
}

impl FetchStats {
    pub(super) fn record_credits(&mut self, credits: &CreditUsage) {
        if credits.remaining.is_some() {
            self.credit_remaining = credits.remaining.clone();
        }
        if credits.used.is_some() {
            self.credit_used = credits.used.clone();
        }
        if credits.last.is_some() {
            self.credit_last = credits.last.clone();
        }
    }

    pub(super) fn credit_summary(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(last) = self.credit_last.as_deref() {
            parts.push(format!("last {last}"));
        }
        if let Some(remaining) = self.credit_remaining.as_deref() {
            parts.push(format!("remaining {remaining}"));
        }
        if let Some(used) = self.credit_used.as_deref() {
            parts.push(format!("used {used}"));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RequestedMarkets {
    featured_markets: Vec<String>,
    pub(super) event_markets: Vec<String>,
}

impl RequestedMarkets {
    pub(super) fn parse(raw: &str) -> Self {
        let mut featured_markets = Vec::new();
        let mut event_markets = Vec::new();
        for market in raw
            .split(',')
            .map(str::trim)
            .filter(|market| !market.is_empty())
        {
            match market {
                "double_chance" => push_unique(&mut event_markets, market),
                "h2h" | "totals" => push_unique(&mut featured_markets, market),
                other => push_unique(&mut featured_markets, other),
            }
        }

        Self {
            featured_markets,
            event_markets,
        }
    }

    pub(super) fn featured_query(&self) -> Option<String> {
        if self.featured_markets.is_empty() {
            None
        } else {
            Some(self.featured_markets.join(","))
        }
    }

    pub(super) fn needs_event_odds(&self) -> bool {
        !self.event_markets.is_empty()
    }
}

fn push_unique(markets: &mut Vec<String>, market: &str) {
    if !markets.iter().any(|existing| existing == market) {
        markets.push(market.to_string());
    }
}

pub(super) fn clean_sport_keys(sport_keys: Vec<String>) -> Vec<String> {
    let cleaned = sport_keys
        .into_iter()
        .map(|sport| sport.trim().to_string())
        .filter(|sport| !sport.is_empty())
        .collect::<Vec<_>>();
    if cleaned.is_empty() {
        vec!["soccer_norway_eliteserien".to_string()]
    } else {
        cleaned
    }
}
