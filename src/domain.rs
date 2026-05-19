#[derive(Debug, Clone, PartialEq)]
pub struct BettingRules {
    pub date: Option<String>,
    pub sport_scope: SportScope,
    pub min_odds: f64,
    pub max_odds: f64,
    pub min_estimated_probability: f64,
    pub min_confidence: f64,
    pub min_edge: f64,
    pub min_expected_value: f64,
    pub pick_count: usize,
}

impl Default for BettingRules {
    fn default() -> Self {
        Self {
            date: None,
            sport_scope: SportScope::Football,
            min_odds: 1.15,
            max_odds: 1.30,
            min_estimated_probability: 0.79,
            min_confidence: 0.65,
            min_edge: 0.015,
            min_expected_value: 0.0,
            pick_count: 5,
        }
    }
}

impl BettingRules {
    pub fn validate(&self) -> Result<(), String> {
        if self.min_odds <= 1.0 {
            return Err("--min-odds must be greater than 1.0".to_string());
        }
        if self.max_odds < self.min_odds {
            return Err("--max-odds must be greater than or equal to --min-odds".to_string());
        }
        validate_unit_interval(self.min_estimated_probability, "--min-probability")?;
        validate_unit_interval(self.min_confidence, "--min-confidence")?;
        if self.pick_count == 0 {
            return Err("--pick-count must be greater than 0".to_string());
        }
        Ok(())
    }

    pub fn filter_by_sport_scope(
        &self,
        candidates: Vec<BetCandidate>,
    ) -> Result<Vec<BetCandidate>, String> {
        let candidates = self
            .sport_scope
            .filter_candidates(candidates, |candidate| &candidate.sport);
        if candidates.is_empty() {
            return Err(format!(
                "no {} candidates were available",
                self.sport_scope.display_name()
            ));
        }
        Ok(candidates)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SportScope {
    Football,
    All,
}

impl SportScope {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "football" | "soccer" | "fotball" => Ok(Self::Football),
            "all" | "any" => Ok(Self::All),
            _ => Err(format!(
                "unsupported sport scope {raw}; expected football or all"
            )),
        }
    }

    pub fn allows_sport(&self, sport: &str) -> bool {
        match self {
            Self::All => true,
            Self::Football => is_football_sport(sport),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Football => "football",
            Self::All => "all-sports",
        }
    }

    fn filter_candidates<T, F>(&self, candidates: Vec<T>, sport_name: F) -> Vec<T>
    where
        F: Fn(&T) -> &str,
    {
        candidates
            .into_iter()
            .filter(|candidate| self.allows_sport(sport_name(candidate)))
            .collect()
    }
}

fn is_football_sport(sport: &str) -> bool {
    let normalized = sport.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || normalized.contains("american")
        || normalized.contains("amerikansk")
        || normalized.contains("australian")
        || normalized.contains("gaelic")
        || normalized.contains("canadian")
    {
        return false;
    }

    normalized
        .split(|ch: char| !ch.is_alphanumeric())
        .any(|part| matches!(part, "football" | "soccer" | "fotball"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct BetCandidate {
    pub id: String,
    pub sport: String,
    pub competition: String,
    pub event: String,
    pub market: String,
    pub selection: String,
    pub norsk_tipping_odds: f64,
    pub model_probability: Option<f64>,
    pub reference_odds: Option<f64>,
    pub confidence: Option<f64>,
    pub starts_at: String,
    pub notes: String,
}

impl BetCandidate {
    pub fn implied_probability(&self) -> f64 {
        1.0 / self.norsk_tipping_odds
    }

    pub fn reference_probability(&self) -> Option<f64> {
        self.reference_odds.map(|odds| 1.0 / odds)
    }

    pub fn has_independent_probability_signal(&self) -> bool {
        self.model_probability.is_some() || self.reference_odds.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProbabilityAssessment {
    pub estimated_probability: f64,
    pub implied_probability: f64,
    pub sources: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueAssessment {
    pub expected_value: f64,
    pub edge: f64,
    pub value_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RiskAssessment {
    pub confidence: f64,
    pub flags: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResearchAssessment {
    pub pages_reviewed: usize,
    pub matched_pages: usize,
    pub positive_mentions: usize,
    pub warning_mentions: usize,
    pub price_hints: Vec<String>,
    pub notes: Vec<String>,
}

impl ResearchAssessment {
    pub fn empty() -> Self {
        Self {
            pages_reviewed: 0,
            matched_pages: 0,
            positive_mentions: 0,
            warning_mentions: 0,
            price_hints: Vec::new(),
            notes: vec!["market research disabled".to_string()],
        }
    }

    pub fn confidence_adjustment(&self) -> f64 {
        let warning_penalty = (self.warning_mentions as f64 * 0.025).min(0.12);
        let positive_bonus = if self.warning_mentions == 0 {
            (self.positive_mentions as f64 * 0.01).min(0.03)
        } else {
            0.0
        };

        positive_bonus - warning_penalty
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FootballContextAssessment {
    pub matched_pages: usize,
    pub categories: Vec<FootballContextCategory>,
    pub confidence_adjustment: f64,
    pub notes: Vec<String>,
}

impl FootballContextAssessment {
    pub fn warning_count(&self) -> usize {
        self.categories
            .iter()
            .filter(|category| category.status == FootballContextStatus::Warning)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FootballContextCategory {
    pub name: String,
    pub status: FootballContextStatus,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootballContextStatus {
    Positive,
    Warning,
    Unknown,
}

impl FootballContextStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Warning => "warning",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedCandidate {
    pub candidate: BetCandidate,
    pub probability: ProbabilityAssessment,
    pub value: ValueAssessment,
    pub risk: RiskAssessment,
    pub research: ResearchAssessment,
    pub football_context: FootballContextAssessment,
    pub learning: LearningAssessment,
    pub score: f64,
    pub rejection_reasons: Vec<String>,
}

impl EvaluatedCandidate {
    pub fn is_bettable(&self) -> bool {
        self.rejection_reasons.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LearningAssessment {
    pub bucket: Option<String>,
    pub settled_samples: usize,
    pub wins: usize,
    pub losses: usize,
    pub hit_rate: Option<f64>,
    pub confidence_adjustment: f64,
    pub notes: Vec<String>,
}

impl LearningAssessment {
    pub fn no_history() -> Self {
        Self {
            bucket: None,
            settled_samples: 0,
            wins: 0,
            losses: 0,
            hit_rate: None,
            confidence_adjustment: 0.0,
            notes: vec!["history: no settled learning data available".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecommendationDecision {
    Bet {
        selected: Box<EvaluatedCandidate>,
        alternatives: Vec<EvaluatedCandidate>,
    },
    BestAvailable {
        reason: String,
        picks: Vec<EvaluatedCandidate>,
    },
    NoBet {
        reason: String,
        reviewed: Vec<EvaluatedCandidate>,
    },
}

fn validate_unit_interval(value: f64, name: &str) -> Result<(), String> {
    if !(0.0..=1.0).contains(&value) {
        return Err(format!("{name} must be between 0.0 and 1.0"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn football_scope_accepts_norwegian_and_english_names() {
        let scope = SportScope::Football;

        assert!(scope.allows_sport("Fotball"));
        assert!(scope.allows_sport("Football"));
        assert!(scope.allows_sport("Soccer"));
        assert!(scope.allows_sport("soccer - england"));
    }

    #[test]
    fn football_scope_rejects_other_sports_and_football_variants() {
        let scope = SportScope::Football;

        assert!(!scope.allows_sport("Tennis"));
        assert!(!scope.allows_sport("Ishockey"));
        assert!(!scope.allows_sport("American Football"));
        assert!(!scope.allows_sport("Amerikansk fotball"));
    }

    #[test]
    fn rules_filter_csv_candidates_to_football_scope() {
        let football = test_candidate("football", "Football");
        let tennis = test_candidate("tennis", "Tennis");

        let filtered = BettingRules::default()
            .filter_by_sport_scope(vec![football.clone(), tennis])
            .expect("football candidate remains");

        assert_eq!(filtered, vec![football]);
    }

    fn test_candidate(id: &str, sport: &str) -> BetCandidate {
        BetCandidate {
            id: id.to_string(),
            sport: sport.to_string(),
            competition: "Competition".to_string(),
            event: "Home - Away".to_string(),
            market: "Main market".to_string(),
            selection: "Home".to_string(),
            norsk_tipping_odds: 1.20,
            model_probability: None,
            reference_odds: None,
            confidence: Some(0.80),
            starts_at: "2026-05-15T18:00:00+02:00".to_string(),
            notes: String::new(),
        }
    }
}
