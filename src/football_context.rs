use crate::domain::{
    BetCandidate, FootballContextAssessment, FootballContextCategory, FootballContextStatus,
};
use crate::research::ResearchDigest;

struct CategoryRule {
    name: &'static str,
    positive: &'static [&'static str],
    warning: &'static [&'static str],
}

pub fn assess_football_context(
    candidate: &BetCandidate,
    digest: Option<&ResearchDigest>,
) -> FootballContextAssessment {
    let mut categories = base_categories();
    let mut matched_pages = 0;
    let mut notes = Vec::new();
    let mut haystacks = Vec::new();

    if !candidate.notes.trim().is_empty() {
        haystacks.push((
            "candidate notes".to_string(),
            candidate.notes.to_lowercase(),
        ));
    }

    if let Some(digest) = digest {
        let terms = candidate_terms(candidate);
        for page in &digest.pages {
            if page.error.is_some() {
                continue;
            }
            let text = page.search_text();
            let term_hits = terms.iter().filter(|term| text.contains(*term)).count();
            if term_hits < 2 {
                continue;
            }
            matched_pages += 1;
            haystacks.push((page.source_name.clone(), context_window(text, &terms)));
        }
    } else {
        notes.push("football context research disabled".to_string());
    }

    for rule in CATEGORY_RULES {
        let Some(category) = categories
            .iter_mut()
            .find(|category| category.name == rule.name)
        else {
            continue;
        };

        for (source, text) in &haystacks {
            collect_matches(
                category,
                source,
                text,
                FootballContextStatus::Warning,
                rule.warning,
            );
            collect_matches(
                category,
                source,
                text,
                FootballContextStatus::Positive,
                rule.positive,
            );
        }
    }

    if matched_pages == 0 {
        notes.push("no candidate-specific football research match found".to_string());
    }

    let warning_count = categories
        .iter()
        .filter(|category| category.status == FootballContextStatus::Warning)
        .count();
    let positive_count = categories
        .iter()
        .filter(|category| category.status == FootballContextStatus::Positive)
        .count();
    let positive_bonus = if warning_count == 0 && matched_pages > 0 {
        (positive_count as f64 * 0.005).min(0.02)
    } else {
        0.0
    };
    let warning_penalty = (warning_count as f64 * 0.03).min(0.12);

    FootballContextAssessment {
        matched_pages,
        categories,
        confidence_adjustment: positive_bonus - warning_penalty,
        notes,
    }
}

fn base_categories() -> Vec<FootballContextCategory> {
    CATEGORY_RULES
        .iter()
        .map(|rule| FootballContextCategory {
            name: rule.name.to_string(),
            status: FootballContextStatus::Unknown,
            evidence: Vec::new(),
        })
        .collect()
}

fn collect_matches(
    category: &mut FootballContextCategory,
    source: &str,
    text: &str,
    status: FootballContextStatus,
    keywords: &[&str],
) {
    for keyword in keywords {
        if !text.contains(keyword) {
            continue;
        }
        if category.status != FootballContextStatus::Warning {
            category.status = status;
        }
        if category.evidence.len() < 3 {
            category
                .evidence
                .push(format!("{source}: {} {keyword}", status.label()));
        }
    }
}

fn candidate_terms(candidate: &BetCandidate) -> Vec<String> {
    let mut terms = Vec::new();
    for raw in [
        candidate.event.as_str(),
        candidate.selection.as_str(),
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

fn context_window(text: &str, terms: &[String]) -> String {
    let words = text
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut context = Vec::new();

    for (index, word) in words.iter().enumerate() {
        if !terms.iter().any(|term| word == term) {
            continue;
        }
        let start = index.saturating_sub(30);
        let end = (index + 31).min(words.len());
        context.extend_from_slice(&words[start..end]);
    }

    if context.is_empty() {
        text.to_string()
    } else {
        context.join(" ")
    }
}

const CATEGORY_RULES: &[CategoryRule] = &[
    CategoryRule {
        name: "Form",
        positive: &["good form", "strong form", "unbeaten", "winning run"],
        warning: &[
            "poor form",
            "bad form",
            "winless",
            "struggling",
            "lost last",
        ],
    },
    CategoryRule {
        name: "Injuries/suspensions",
        positive: &[
            "full squad",
            "no fresh injury",
            "clean bill",
            "key player returns",
        ],
        warning: &[
            "injury",
            "injured",
            "doubtful",
            "suspended",
            "suspension",
            "ruled out",
        ],
    },
    CategoryRule {
        name: "Motivation",
        positive: &[
            "must win",
            "title race",
            "promotion",
            "relegation battle",
            "europe",
        ],
        warning: &[
            "dead rubber",
            "nothing to play",
            "already qualified",
            "already relegated",
        ],
    },
    CategoryRule {
        name: "Schedule/travel",
        positive: &["well-rested", "fresh legs", "full week rest"],
        warning: &[
            "short rest",
            "fatigue",
            "travel fatigue",
            "congested",
            "midweek",
        ],
    },
    CategoryRule {
        name: "Market context",
        positive: &[
            "value",
            "mispriced",
            "good price",
            "market support",
            "odds shortening",
        ],
        warning: &[
            "odds drifting",
            "drift",
            "trap",
            "avoid",
            "price too short",
            "no bet",
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::{ResearchPage, ResearchSignal};

    #[test]
    fn leaves_unmatched_research_unknown_without_boost() {
        let digest = ResearchDigest {
            pages: vec![page(
                "preview",
                "Tennis final",
                "Good form and full squad are mentioned for another event.",
            )],
        };

        let assessment = assess_football_context(&candidate(""), Some(&digest));

        assert_eq!(assessment.matched_pages, 0);
        assert_eq!(assessment.confidence_adjustment, 0.0);
        assert!(
            assessment
                .categories
                .iter()
                .all(|category| category.status == FootballContextStatus::Unknown)
        );
    }

    #[test]
    fn downgrades_candidate_specific_warning_context() {
        let digest = ResearchDigest {
            pages: vec![page(
                "preview",
                "Rosenborg Brann preview",
                "Rosenborg - Brann has injury news, short rest and could be a dead rubber.",
            )],
        };

        let assessment = assess_football_context(&candidate(""), Some(&digest));

        assert_eq!(assessment.matched_pages, 1);
        assert!(assessment.confidence_adjustment < 0.0);
        assert!(assessment.warning_count() >= 3);
    }

    #[test]
    fn ignores_warning_terms_far_from_candidate_context() {
        let digest = ResearchDigest {
            pages: vec![page(
                "preview",
                "Rosenborg Brann preview",
                "Rosenborg - Brann has a stable preview. analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis analysis Another unrelated match has injury news and short rest.",
            )],
        };

        let assessment = assess_football_context(&candidate(""), Some(&digest));

        assert_eq!(assessment.matched_pages, 1);
        assert_eq!(assessment.warning_count(), 0);
    }

    #[test]
    fn uses_candidate_notes_as_supplied_context() {
        let assessment = assess_football_context(&candidate("strong form"), None);

        assert_eq!(assessment.matched_pages, 0);
        assert_eq!(assessment.confidence_adjustment, 0.0);
        assert!(
            assessment
                .categories
                .iter()
                .any(|category| category.status == FootballContextStatus::Positive)
        );
    }

    fn candidate(notes: &str) -> BetCandidate {
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
            notes: notes.to_string(),
        }
    }

    fn page(source_name: &str, title: &str, text: &str) -> ResearchPage {
        ResearchPage::new(
            source_name.to_string(),
            "https://example.test".to_string(),
            title.to_string(),
            text.to_string(),
            vec![ResearchSignal::Warning("injury".to_string())],
            None,
        )
    }
}
