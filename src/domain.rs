#[derive(Debug, Clone, PartialEq)]
pub struct BettingRules {
    pub date: Option<String>,
    pub min_odds: f64,
    pub max_odds: f64,
    pub min_estimated_probability: f64,
    pub min_confidence: f64,
    pub min_edge: f64,
    pub min_expected_value: f64,
}

impl Default for BettingRules {
    fn default() -> Self {
        Self {
            date: None,
            min_odds: 1.15,
            max_odds: 1.30,
            min_estimated_probability: 0.79,
            min_confidence: 0.65,
            min_edge: 0.015,
            min_expected_value: 0.0,
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
        Ok(())
    }
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
pub struct EvaluatedCandidate {
    pub candidate: BetCandidate,
    pub probability: ProbabilityAssessment,
    pub value: ValueAssessment,
    pub risk: RiskAssessment,
    pub research: ResearchAssessment,
    pub score: f64,
    pub rejection_reasons: Vec<String>,
}

impl EvaluatedCandidate {
    pub fn is_bettable(&self) -> bool {
        self.rejection_reasons.is_empty()
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
