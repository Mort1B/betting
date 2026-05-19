pub(super) const EXPLORER_INSTRUCTIONS: &str = r#"You are the Explorer agent for a daily betting workflow.
Use the supplied deterministic report only. Identify the strongest probability, context, confidence, and research signals for the top candidates.
For every candidate, summarize supplied evidence for form, injuries/suspensions, lineups/rotation, motivation, schedule/travel pressure, weather/venue, market context, the learning note, research matches, and optional model/reference evidence.
Treat unknown football checklist items as missing evidence, not as positive or negative facts. Do not infer team news, motivation, injuries, odds, probabilities, sources, or results beyond the supplied report. Keep output concise."#;

pub(super) const REVIEWER_INSTRUCTIONS: &str = r#"You are the Reviewer agent.
Challenge the Explorer and deterministic ranking. Look for overclaiming, weak football context evidence, stale or missing research, unresolved team news, underestimated form/injury/motivation/schedule risk, and cases where a bet is likely but not supported enough.
Check that the learning note is not overstated: insufficient history or no settled data must not become a confidence claim.
Return concise bullets with approve/question/reject style judgments for each top candidate.
Do not invent facts, do not add unsupplied football context, do not treat slack odds as strict picks, and do not recommend bets outside the supplied Norsk Tipping research band."#;

pub(super) const RISK_MANAGER_INSTRUCTIONS: &str = r#"You are the Risk Manager agent.
Identify downside risks, confidence concerns, missing data, and no-bet triggers. Treat gambling outcomes as uncertain and never imply a guaranteed win.
Downgrade or question candidates when injuries, suspensions, lineup, rotation, motivation, schedule, weather, venue, market context, or learning support is unresolved, negative, or insufficient in the supplied report.
Preserve deterministic fallback status and rejection reasons; do not turn a fallback candidate into a strict recommendation.
Return concise risk notes for each top candidate and say whether any candidate should be downgraded.
Use only supplied facts."#;

pub(super) const OUTPUT_WRITER_INSTRUCTIONS: &str = r#"You are the Output Writer agent.
Write the final user-facing daily report using the deterministic report plus the Explorer, Reviewer, and Risk Manager outputs.
The output must include the top 5 candidates when available, preserving deterministic rank order. For each candidate include: sport/competition, event, market, selection, Norsk Tipping odds, probability/confidence basis, football context checklist summary, learning note, reference-market comparison only when supplied, main risks, strict rules status, and confidence score out of 100.
If the deterministic report says TOP 5 CANDIDATES, preserve those five ranked candidates and their fallback warnings instead of converting the report to NO BET.
If the deterministic report says NO BET because no viable candidates were supplied, output NO BET and explain why.
Keep unknown football context visible as unknown. Keep it practical, concise, and suitable for an iPhone notification/page. Do not invent facts."#;

const MAX_PRIOR_AGENT_CONTEXT_CHARS: usize = 2_400;

pub(super) fn agent_input(compact_report: &str, prior_outputs: &[(&str, &str)]) -> String {
    let mut input = format!("Compact deterministic report:\n\n{compact_report}");
    for (label, output) in prior_outputs {
        input.push_str("\n\n");
        input.push_str(label);
        input.push_str(":\n\n");
        input.push_str(&bounded_agent_context(output));
    }
    input
}

fn bounded_agent_context(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.chars().count() <= MAX_PRIOR_AGENT_CONTEXT_CHARS {
        return trimmed.to_string();
    }

    let excerpt = trimmed
        .chars()
        .take(MAX_PRIOR_AGENT_CONTEXT_CHARS)
        .collect::<String>();
    format!("{excerpt}\n[truncated to {MAX_PRIOR_AGENT_CONTEXT_CHARS} chars]")
}

pub(super) fn compact_deterministic_report(deterministic_report: &str) -> String {
    let mut output = String::new();
    let mut kept = 0;
    for line in deterministic_report.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if keep_compact_report_line(trimmed) {
            output.push_str(trimmed);
            output.push('\n');
            kept += 1;
        }
    }

    if kept == 0 {
        deterministic_report.trim().to_string()
    } else {
        output.trim_end().to_string()
    }
}

fn keep_compact_report_line(line: &str) -> bool {
    COMPACT_REPORT_PREFIXES
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

#[cfg(test)]
fn estimated_input_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

const COMPACT_REPORT_PREFIXES: &[&str] = &[
    "Rules:",
    "Date filter:",
    "Scope:",
    "Pick history:",
    "Source coverage:",
    "Missing context:",
    "Learning summary:",
    "Decision:",
    "Reason:",
    "These are",
    "Top ",
    "Closest reviewed candidates:",
    "No viable bets",
    "#",
    "Sport:",
    "Event:",
    "Competition:",
    "Starts at:",
    "Market:",
    "Selection:",
    "Norsk Tipping odds:",
    "Reference market odds:",
    "Norsk Tipping comparison:",
    "Estimated probability:",
    "Norsk Tipping implied probability:",
    "Expected value:",
    "Edge:",
    "Confidence:",
    "Confidence score:",
    "Strict rules status:",
    "Research:",
    "Football context:",
    "Learning:",
    "Football context checklist:",
    "- ",
    "Score:",
    "Probability sources:",
    "Probability notes:",
    "Risk flags:",
    "Notes:",
    "Research price hints:",
    "Research notes:",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_role_prompts_preserve_context_and_learning_constraints() {
        assert!(EXPLORER_INSTRUCTIONS.contains("learning note"));
        assert!(EXPLORER_INSTRUCTIONS.contains("unknown football checklist"));
        assert!(REVIEWER_INSTRUCTIONS.contains("insufficient history"));
        assert!(RISK_MANAGER_INSTRUCTIONS.contains("fallback status"));
        assert!(OUTPUT_WRITER_INSTRUCTIONS.contains("top 5 candidates"));
        assert!(OUTPUT_WRITER_INSTRUCTIONS.contains("learning note"));
        assert!(OUTPUT_WRITER_INSTRUCTIONS.contains("Do not invent facts"));
    }

    #[test]
    fn compact_report_keeps_required_candidate_fields() {
        let compact = compact_deterministic_report(COMPACT_FIXTURE);

        for required in [
            "Decision: TOP 2 CANDIDATES",
            "#1 Rosenborg - Brann",
            "Norsk Tipping odds: 1.27",
            "Strict rules status: pass",
            "- Lineup/rotation: positive: candidate notes: lineups stable",
            "Learning: history: no settled learning data available",
            "Risk flags: 1 research warning mention(s)",
            "Research notes:",
            "- source error: Example feed timeout",
            "#2 Arsenal - Everton",
            "Strict rules status: fallback candidate",
        ] {
            assert!(
                compact.contains(required),
                "missing compact field: {required}"
            );
        }
    }

    #[test]
    fn compact_report_is_smaller_than_full_report_replay() {
        let compact = compact_deterministic_report(COMPACT_FIXTURE);
        let raw_replay = format!(
            "Deterministic betting report:\n\n{0}\n\nExplorer output:\n\nx\n\nReviewer output:\n\nx",
            COMPACT_FIXTURE
        );
        let compact_replay = agent_input(&compact, &[("Explorer output", "x")]);

        assert!(estimated_input_tokens(&compact_replay) < estimated_input_tokens(&raw_replay));
    }

    const COMPACT_FIXTURE: &str = r#"Daily betting agent recommendation
==================================

Rules: Norsk Tipping preferred odds 1.10-1.30, hard research ceiling 1.35, min probability 79.00%, min confidence 65.00%, min edge 1.50 pp when model/reference data exists
Scope: football | Pick target: 2
Source coverage: reviewed up to 10 page(s); matched 1/2 pick(s); warnings on 1; source errors 1
Learning summary: history: no settled learning data available

Decision: TOP 2 CANDIDATES
Reason: fallback fill
Top 2 candidates:

#1 Rosenborg - Brann
Sport: Football
Competition: Eliteserien
Market: Double chance
Selection: Rosenborg or draw
Norsk Tipping odds: 1.27
Estimated probability: 83.50%
Confidence score: 78/100
Strict rules status: pass
Learning: history: no settled learning data available
Football context checklist:
- Lineup/rotation: positive: candidate notes: lineups stable
Risk flags: 1 research warning mention(s)
Research notes:
- source error: Example feed timeout
Explanation: verbose repeated explanation that is not needed in compact input

#2 Arsenal - Everton
Sport: Football
Competition: Premier League
Market: Match winner
Selection: Arsenal
Norsk Tipping odds: 1.34
Estimated probability: 84.00%
Confidence score: 76/100
Strict rules status: fallback candidate (Norsk Tipping odds 1.34 are above preferred ceiling 1.30; slack fallback only)
Learning: history: no settled learning data available
Explanation: another long generated sentence"#;
}
