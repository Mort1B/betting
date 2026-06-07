use crate::domain::{BetCandidate, ResearchAssessment};

#[derive(Debug, Clone, PartialEq)]
pub struct ResearchDigest {
    pub pages: Vec<ResearchPage>,
}

impl ResearchDigest {
    pub fn empty() -> Self {
        Self { pages: Vec::new() }
    }

    pub fn source_error_count(&self) -> usize {
        self.pages
            .iter()
            .filter(|page| page.error.is_some())
            .count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResearchPage {
    pub source_name: String,
    pub url: String,
    pub title: String,
    pub text: String,
    pub signals: Vec<ResearchSignal>,
    pub error: Option<String>,
    search_text: String,
}

impl ResearchPage {
    pub fn new(
        source_name: String,
        url: String,
        title: String,
        text: String,
        signals: Vec<ResearchSignal>,
        error: Option<String>,
    ) -> Self {
        let search_text = normalize_search_text(&title, &text);
        Self {
            source_name,
            url,
            title,
            text,
            signals,
            error,
            search_text,
        }
    }

    pub fn source_error(source_name: String, url: String, error: String) -> Self {
        Self::new(
            source_name.clone(),
            url,
            source_name,
            String::new(),
            Vec::new(),
            Some(error),
        )
    }

    pub fn search_text(&self) -> &str {
        &self.search_text
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResearchSignal {
    Positive(String),
    Warning(String),
    PriceHint(String),
}

pub fn assess_candidate_research(
    candidate: &BetCandidate,
    digest: Option<&ResearchDigest>,
) -> ResearchAssessment {
    let Some(digest) = digest else {
        return ResearchAssessment::empty();
    };

    let mut matched_pages = 0;
    let mut positive_mentions = 0;
    let mut warning_mentions = 0;
    let mut price_hints = Vec::new();
    let mut notes = Vec::new();
    let source_error_count = digest.source_error_count();
    let pages_reviewed = digest.pages.len().saturating_sub(source_error_count);
    let terms = candidate_terms(candidate);
    let event_terms = split_terms(&candidate.event);

    for page in &digest.pages {
        if page.error.is_some() {
            continue;
        }

        if !has_candidate_specific_event_match(page.search_text(), &event_terms) {
            continue;
        }
        let term_hits = terms
            .iter()
            .filter(|term| contains_term(page.search_text(), term))
            .count();
        if term_hits < 2 {
            continue;
        }

        matched_pages += 1;
        let page_note = format!("{} matched {} candidate terms", page.source_name, term_hits);
        notes.push(page_note);

        for signal in &page.signals {
            match signal {
                ResearchSignal::Positive(text) => {
                    positive_mentions += 1;
                    notes.push(format!("positive: {}: {text}", page.source_name));
                }
                ResearchSignal::Warning(text) => {
                    warning_mentions += 1;
                    notes.push(format!("warning: {}: {text}", page.source_name));
                }
                ResearchSignal::PriceHint(text) => {
                    price_hints.push(format!("{}: {text}", page.source_name));
                }
            }
        }
    }

    if notes.is_empty() {
        notes.push("no relevant research matches found".to_string());
    }

    ResearchAssessment {
        pages_reviewed,
        source_error_count,
        matched_pages,
        positive_mentions,
        warning_mentions,
        price_hints,
        notes,
    }
}

pub fn analyze_text(source_name: &str, text: &str) -> Vec<ResearchSignal> {
    let lower = text.to_lowercase();
    let mut signals = Vec::new();

    for keyword in [
        "value",
        "mispriced",
        "wrong odds",
        "best bet",
        "banker",
        "strong pick",
        "good price",
    ] {
        if lower.contains(keyword) {
            signals.push(ResearchSignal::Positive(keyword.to_string()));
        }
    }

    for keyword in [
        "avoid",
        "trap",
        "injury",
        "suspended",
        "doubtful",
        "no bet",
        "resting",
    ] {
        if lower.contains(keyword) {
            signals.push(ResearchSignal::Warning(keyword.to_string()));
        }
    }

    for odd in extract_decimal_odds(&lower) {
        if (1.05..=3.50).contains(&odd) {
            signals.push(ResearchSignal::PriceHint(format!(
                "decimal odds mention {odd:.2} near {source_name}"
            )));
        }
    }

    signals
}

fn candidate_terms(candidate: &BetCandidate) -> Vec<String> {
    let mut terms = Vec::new();
    for raw in [
        candidate.event.as_str(),
        candidate.selection.as_str(),
        candidate.market.as_str(),
        candidate.competition.as_str(),
    ] {
        for lower in split_terms(raw) {
            if !terms.contains(&lower) {
                terms.push(lower);
            }
        }
    }
    terms
}

fn split_terms(raw: &str) -> Vec<String> {
    raw.split(|ch: char| !ch.is_alphanumeric())
        .map(str::trim)
        .filter(|part| part.len() >= 4)
        .map(str::to_lowercase)
        .collect()
}

fn has_candidate_specific_event_match(text: &str, event_terms: &[String]) -> bool {
    if event_terms.len() >= 2 {
        return event_terms
            .iter()
            .filter(|term| contains_term(text, term))
            .count()
            >= 2;
    }
    event_terms.iter().any(|term| contains_term(text, term))
}

fn contains_term(text: &str, term: &str) -> bool {
    text.split(|ch: char| !ch.is_alphanumeric())
        .any(|word| word == term)
}

fn normalize_search_text(title: &str, text: &str) -> String {
    format!("{title} {text}").to_lowercase()
}

fn extract_decimal_odds(text: &str) -> Vec<f64> {
    let mut odds = Vec::new();
    for token in text.split(|ch: char| !(ch.is_ascii_digit() || ch == '.')) {
        let Some(dot_index) = token.find('.') else {
            continue;
        };
        if dot_index == 0 || dot_index + 1 >= token.len() {
            continue;
        }
        if let Ok(value) = token.parse::<f64>() {
            odds.push(value);
        }
    }
    odds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_research_signals() {
        let signals = analyze_text(
            "test",
            "Rosenborg looks like value at 1.27 but has an injury concern.",
        );

        assert!(signals.contains(&ResearchSignal::Positive("value".to_string())));
        assert!(signals.contains(&ResearchSignal::Warning("injury".to_string())));
        assert!(
            signals
                .iter()
                .any(|signal| matches!(signal, ResearchSignal::PriceHint(_)))
        );
    }

    #[test]
    fn counts_source_errors_without_candidate_note_spam() {
        let digest = ResearchDigest {
            pages: vec![ResearchPage::source_error(
                "blocked source".to_string(),
                "https://example.test".to_string(),
                "returned 403".to_string(),
            )],
        };

        let assessment = assess_candidate_research(&candidate(), Some(&digest));

        assert_eq!(assessment.pages_reviewed, 0);
        assert_eq!(assessment.source_error_count, 1);
        assert_eq!(assessment.notes, vec!["no relevant research matches found"]);
    }

    #[test]
    fn ignores_generic_pages_without_event_terms() {
        let digest = ResearchDigest {
            pages: vec![ResearchPage::new(
                "SportyTrader football tips".to_string(),
                "https://example.test".to_string(),
                "Football betting tips".to_string(),
                "Premier League value tips mention over goals and draw markets.".to_string(),
                vec![ResearchSignal::Positive("value".to_string())],
                None,
            )],
        };

        let assessment = assess_candidate_research(&candidate(), Some(&digest));

        assert_eq!(assessment.matched_pages, 0);
        assert_eq!(assessment.positive_mentions, 0);
    }

    #[test]
    fn matches_pages_with_both_event_sides() {
        let digest = ResearchDigest {
            pages: vec![ResearchPage::new(
                "preview".to_string(),
                "https://example.test".to_string(),
                "Rosenborg Brann preview".to_string(),
                "Rosenborg vs Brann looks like value at 1.27.".to_string(),
                vec![ResearchSignal::Positive("value".to_string())],
                None,
            )],
        };

        let assessment = assess_candidate_research(&candidate(), Some(&digest));

        assert_eq!(assessment.matched_pages, 1);
        assert_eq!(assessment.positive_mentions, 1);
    }

    fn candidate() -> BetCandidate {
        BetCandidate {
            id: "c1".to_string(),
            sport: "Football".to_string(),
            competition: "Eliteserien".to_string(),
            event: "Rosenborg - Brann".to_string(),
            market: "Double chance".to_string(),
            selection: "Rosenborg or draw".to_string(),
            norsk_tipping_odds: 1.22,
            model_probability: None,
            reference_odds: None,
            confidence: Some(0.75),
            starts_at: "2026-05-15T18:00:00+02:00".to_string(),
            notes: String::new(),
        }
    }
}
