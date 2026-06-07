use super::*;
use serde_json::json;
use std::collections::VecDeque;

#[test]
fn extracts_top_level_output_text() {
    let value = json!({"output_text": "hello"});
    assert_eq!(extract_output_text(&value), Some("hello".to_string()));
}

#[test]
fn extracts_nested_response_text() {
    let value = json!({
        "output": [{
            "content": [
                {"type": "output_text", "text": "hello"},
                {"type": "output_text", "text": "world"}
            ]
        }]
    });
    assert_eq!(
        extract_output_text(&value),
        Some("hello\nworld".to_string())
    );
}

#[test]
fn rejects_incomplete_openai_response() {
    let value = json!({
        "status": "incomplete",
        "incomplete_details": {"reason": "max_output_tokens"},
        "output_text": "#1 partial"
    });

    assert_eq!(
        incomplete_response_reason(&value).as_deref(),
        Some("max_output_tokens")
    );
}

#[test]
fn skips_ai_for_empty_no_bet_report() {
    let report = "Daily betting agent recommendation\n\nDecision: NO BET\n\nNo viable bets available; no candidates were available to rank.\n";
    let options = AiOptions {
        enabled: true,
        ..AiOptions::default()
    };

    assert_eq!(run_ai_workflow(report, &options), Ok(None));
}

#[test]
fn ai_workflow_keeps_four_roles_with_compact_inputs() {
    let mut client = MockAiClient::new([
        "explorer summary",
        "reviewer challenge",
        "risk notes",
        "Football data provider: API-Football disabled: football data API key not configured\n#1 Rosenborg - Brann\nfinal first pick\n\n#2 Arsenal - Everton\nfinal second pick",
    ]);

    let report = run_ai_workflow_with_client(WORKFLOW_FIXTURE, &mut client)
        .expect("mock workflow should succeed");

    assert_eq!(report.explorer, "explorer summary");
    assert_eq!(report.reviewer, "reviewer challenge");
    assert_eq!(report.risk_manager, "risk notes");
    assert_eq!(
        report.final_output,
        "Football data provider: API-Football disabled: football data API key not configured\n#1 Rosenborg - Brann\nfinal first pick\n\n#2 Arsenal - Everton\nfinal second pick"
    );
    assert_eq!(
        client
            .calls
            .iter()
            .map(|call| call.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Explorer", "Reviewer", "Risk manager", "Output writer"]
    );

    let explorer = &client.calls[0].input;
    assert!(explorer.contains("Compact deterministic report:"));
    assert!(explorer.contains("Decision: TOP 2 CANDIDATES"));
    assert!(explorer.contains("Football data provider: API-Football disabled"));
    assert!(explorer.contains("#1 Rosenborg - Brann"));
    assert!(explorer.contains("Strict rules status: fallback candidate"));
    assert!(explorer.contains("Football context checklist:"));
    assert!(explorer.contains("Learning: history: no settled learning data available"));
    assert!(!explorer.contains("Explanation:"));
    assert!(!explorer.contains("Explorer output:"));

    assert!(
        client.calls[1]
            .input
            .contains("Explorer output:\n\nexplorer summary")
    );
    assert!(!client.calls[1].input.contains("Reviewer output:"));
    assert!(
        client.calls[2]
            .input
            .contains("Reviewer output:\n\nreviewer challenge")
    );
    assert!(
        client.calls[3]
            .input
            .contains("Risk manager output:\n\nrisk notes")
    );
}

#[test]
fn rejects_ai_final_output_that_omits_ranked_candidates() {
    let mut client = MockAiClient::new([
        "explorer summary",
        "reviewer challenge",
        "risk notes",
        "#1 Rosenborg - Brann\ncomplete first pick only",
    ]);

    let error = run_ai_workflow_with_client(WORKFLOW_FIXTURE, &mut client)
        .expect_err("final output should be rejected");

    assert!(error.contains("omitted ranked candidates"));
}

#[test]
fn rejects_ai_final_output_that_omits_provider_status() {
    let mut client = MockAiClient::new([
        "explorer summary",
        "reviewer challenge",
        "risk notes",
        "#1 Rosenborg - Brann\nfinal first pick\n\n#2 Arsenal - Everton\nfinal second pick",
    ]);

    let error = run_ai_workflow_with_client(WORKFLOW_FIXTURE, &mut client)
        .expect_err("final output should be rejected");

    assert!(error.contains("omitted summary line"));
}

struct MockAiClient {
    outputs: VecDeque<String>,
    calls: Vec<MockCall>,
}

struct MockCall {
    name: String,
    input: String,
}

impl MockAiClient {
    fn new(outputs: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            outputs: outputs.into_iter().map(str::to_string).collect(),
            calls: Vec::new(),
        }
    }
}

impl AiAgentClient for MockAiClient {
    fn call_agent(
        &mut self,
        name: &str,
        _instructions: &str,
        input: &str,
    ) -> Result<String, String> {
        self.calls.push(MockCall {
            name: name.to_string(),
            input: input.to_string(),
        });
        self.outputs
            .pop_front()
            .ok_or_else(|| format!("missing mock output for {name}"))
    }
}

const WORKFLOW_FIXTURE: &str = r#"Daily betting agent recommendation
==================================

Rules: Norsk Tipping preferred odds 1.10-1.30, hard research ceiling 1.35, min probability 79.00%, min confidence 65.00%, min edge 1.50 pp when model/reference data exists
Scope: football | Pick target: 2
Football data provider: API-Football disabled: football data API key not configured
Decision: TOP 2 CANDIDATES
Reason: fallback fill
Top 2 candidates:

#1 Rosenborg - Brann
Sport: Football
Competition: Eliteserien
Starts at: 2026-05-15T18:00:00+02:00
Kickoff time: 18:00 on 2026-05-15 (Oslo time)
Market: Double chance
Selection: Rosenborg or draw
Norsk Tipping odds: 1.27
Estimated probability: 83.50%
Confidence score: 78/100
Strict rules status: pass
Learning: history: no settled learning data available
Football context checklist:
- Form: positive: candidate notes: strong form
Explanation: verbose text should not be replayed to AI roles

#2 Arsenal - Everton
Sport: Football
Competition: Premier League
Market: Match winner
Selection: Arsenal
Norsk Tipping odds: 1.34
Estimated probability: 84.00%
Confidence score: 76/100
Strict rules status: fallback candidate (Norsk Tipping odds 1.34 are above preferred ceiling 1.30; slack fallback only)
Learning: history: no settled learning data available"#;
