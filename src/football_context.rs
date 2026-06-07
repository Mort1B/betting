use crate::domain::{
    BetCandidate, FootballContextAssessment, FootballContextCategory, FootballContextStatus,
};
use crate::research::ResearchDigest;

mod api_evidence;
#[cfg(test)]
mod tests;

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
        let event_terms = split_terms(&candidate.event);
        for page in &digest.pages {
            if page.error.is_some() {
                continue;
            }
            let text = page.search_text();
            if !has_candidate_specific_event_match(text, &event_terms) {
                continue;
            }
            let term_hits = terms
                .iter()
                .filter(|term| contains_term(text, term))
                .count();
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
    api_evidence::append_unknown_api_evidence(&mut categories, candidate);

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
        if should_skip_match(&category.name, text, status, keyword) {
            continue;
        }
        if category.status == FootballContextStatus::Warning
            && status != FootballContextStatus::Warning
        {
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

fn should_skip_match(
    category_name: &str,
    text: &str,
    status: FootballContextStatus,
    keyword: &str,
) -> bool {
    category_name == "Injuries/suspensions"
        && status == FootballContextStatus::Warning
        && keyword == "injury"
        && INJURY_NEGATION_PHRASES
            .iter()
            .any(|phrase| text.contains(phrase))
        && !INJURY_HARD_WARNING_PHRASES
            .iter()
            .any(|phrase| text.contains(phrase))
}

fn candidate_terms(candidate: &BetCandidate) -> Vec<String> {
    let mut terms = Vec::new();
    for raw in [
        candidate.event.as_str(),
        candidate.selection.as_str(),
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
        positive: &[
            "good form",
            "strong form",
            "unbeaten",
            "winning run",
            "opponent vulnerable form",
        ],
        warning: &[
            "poor form",
            "bad form",
            "winless",
            "struggling",
            "lost last",
            "opponent strong form",
        ],
    },
    CategoryRule {
        name: "Injuries/suspensions",
        positive: &[
            "full squad",
            "no fresh injury",
            "no listed absences",
            "clean bill",
            "key player returns",
            "opponent absences",
        ],
        warning: &[
            "selected team absences",
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
            "europe place",
            "european place",
            "european spot",
            "european qualification",
            "champions league",
            "europa league",
        ],
        warning: &[
            "dead rubber",
            "nothing to play",
            "already qualified",
            "already relegated",
            "opponent motivation risk",
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
            "market agreement tight",
        ],
        warning: &[
            "odds drifting",
            "drift",
            "trap",
            "avoid",
            "price too short",
            "no bet",
            "market disagreement high",
            "single reference source",
        ],
    },
];

const INJURY_NEGATION_PHRASES: &[&str] = &[
    "no fresh injury",
    "no listed absences",
    "clean bill",
    "full squad",
];

const INJURY_HARD_WARNING_PHRASES: &[&str] = &[
    "injured",
    "doubtful",
    "suspended",
    "suspension",
    "ruled out",
    "selected team absences",
];
