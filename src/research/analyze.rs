use crate::domain::{BetCandidate, ResearchAssessment};

#[derive(Debug, Clone, PartialEq)]
pub struct ResearchDigest {
    pub pages: Vec<ResearchPage>,
}

impl ResearchDigest {
    pub fn empty() -> Self {
        Self { pages: Vec::new() }
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
    let terms = candidate_terms(candidate);

    for page in &digest.pages {
        if let Some(error) = &page.error {
            notes.push(format!("source error: {}: {error}", page.source_name));
            continue;
        }

        let term_hits = terms
            .iter()
            .filter(|term| page.search_text().contains(*term))
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
        pages_reviewed: digest.pages.len(),
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
        for part in raw
            .split(|ch: char| !ch.is_alphanumeric())
            .map(str::trim)
            .filter(|part| part.len() >= 4)
        {
            let lower = part.to_lowercase();
            if !terms.contains(&lower) {
                terms.push(lower);
            }
        }
    }
    terms
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
    fn exposes_source_errors_as_research_notes() {
        let digest = ResearchDigest {
            pages: vec![ResearchPage::source_error(
                "blocked source".to_string(),
                "https://example.test".to_string(),
                "returned 403".to_string(),
            )],
        };

        let assessment = assess_candidate_research(&candidate(), Some(&digest));

        assert!(
            assessment
                .notes
                .iter()
                .any(|note| note.contains("source error: blocked source: returned 403"))
        );
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
