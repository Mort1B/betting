use crate::domain::{BetCandidate, FootballContextCategory, FootballContextStatus};

pub(super) fn append_unknown_api_evidence(
    categories: &mut [FootballContextCategory],
    candidate: &BetCandidate,
) {
    let notes = candidate.notes.to_lowercase();
    let fixture_unmatched = notes.contains("api-football fixture not matched");
    let fixture_matched = notes.contains("api-football fixture matched");
    let fixture_skipped =
        notes.contains("api-football fixture matched but context enrichment skipped");

    push_unknown_evidence(
        categories,
        "Form",
        &notes,
        &[
            (
                "api-football form checked",
                "API-Football: form checked but no clear positive/warning signal",
            ),
            (
                "api-football form:",
                "API-Football: form checked but no clear positive/warning signal",
            ),
        ],
    );
    push_unknown_evidence(
        categories,
        "Injuries/suspensions",
        &notes,
        &[
            (
                "api-football availability coverage unavailable",
                "API-Football: injury/suspension coverage unavailable",
            ),
            (
                "api-football availability coverage not confirmed",
                "API-Football: injury/suspension coverage not confirmed",
            ),
        ],
    );
    push_unknown_evidence(
        categories,
        "Motivation",
        &notes,
        &[
            (
                "api-football table coverage unavailable",
                "API-Football: standings/motivation coverage unavailable",
            ),
            (
                "api-football table coverage not confirmed",
                "API-Football: standings/motivation coverage not confirmed",
            ),
            (
                "api-football table checked",
                "API-Football: standings checked but no clear motivation signal",
            ),
        ],
    );
    push_unknown_evidence(
        categories,
        "Schedule/travel",
        &notes,
        &[
            (
                "schedule checked no short-rest signal",
                "API-Football: schedule checked, no short-rest signal",
            ),
            (
                "schedule checked no rest-day signal",
                "API-Football: schedule checked, no rest-day signal",
            ),
        ],
    );

    if fixture_unmatched {
        for category_name in [
            "Form",
            "Injuries/suspensions",
            "Motivation",
            "Schedule/travel",
        ] {
            push_unknown_category_evidence(
                categories,
                category_name,
                "API-Football: fixture not matched, category unavailable",
            );
        }
    } else if fixture_skipped {
        for category_name in [
            "Form",
            "Injuries/suspensions",
            "Motivation",
            "Schedule/travel",
        ] {
            push_unknown_category_evidence(
                categories,
                category_name,
                "API-Football: fixture matched, context enrichment skipped by cap",
            );
        }
    } else if fixture_matched {
        push_unknown_category_evidence(
            categories,
            "Motivation",
            "API-Football: fixture matched, no clear standings motivation signal",
        );
    }

    if candidate.reference_odds.is_none() {
        push_unknown_category_evidence(
            categories,
            "Market context",
            "reference odds: no candidate-level reference price matched",
        );
    }
}

fn push_unknown_evidence(
    categories: &mut [FootballContextCategory],
    category_name: &str,
    notes: &str,
    patterns: &[(&str, &str)],
) {
    for (pattern, evidence) in patterns {
        if notes.contains(pattern) {
            push_unknown_category_evidence(categories, category_name, evidence);
            return;
        }
    }
}

fn push_unknown_category_evidence(
    categories: &mut [FootballContextCategory],
    category_name: &str,
    evidence: &str,
) {
    let Some(category) = categories
        .iter_mut()
        .find(|category| category.name == category_name)
    else {
        return;
    };
    if category.status == FootballContextStatus::Unknown
        && !category
            .evidence
            .iter()
            .any(|existing| existing == evidence)
        && category.evidence.len() < 3
    {
        category.evidence.push(evidence.to_string());
    }
}
