mod prompts;

use std::env;
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{Value, json};

use prompts::{
    EXPLORER_INSTRUCTIONS, OUTPUT_WRITER_INSTRUCTIONS, REVIEWER_INSTRUCTIONS,
    RISK_MANAGER_INSTRUCTIONS, agent_input, compact_deterministic_report,
};

#[derive(Debug, Clone, PartialEq)]
pub struct AiOptions {
    pub enabled: bool,
    pub model: String,
    pub max_output_tokens: u32,
}

impl Default for AiOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            model: env::var("BETTING_OPENAI_MODEL").unwrap_or_else(|_| "gpt-5.5".to_string()),
            max_output_tokens: 900,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AiWorkflowReport {
    pub explorer: String,
    pub reviewer: String,
    pub risk_manager: String,
    pub final_output: String,
}

pub fn run_ai_workflow(
    deterministic_report: &str,
    options: &AiOptions,
) -> Result<Option<AiWorkflowReport>, String> {
    if !options.enabled {
        return Ok(None);
    }
    if should_skip_ai_for_empty_no_bet_report(deterministic_report) {
        return Ok(None);
    }

    let mut client = OpenAiClient::new(options)?;
    let report = run_ai_workflow_with_client(deterministic_report, &mut client)?;
    Ok(Some(report))
}

fn run_ai_workflow_with_client(
    deterministic_report: &str,
    client: &mut impl AiAgentClient,
) -> Result<AiWorkflowReport, String> {
    let compact_report = compact_deterministic_report(deterministic_report);
    let explorer = client.call_agent(
        "Explorer",
        EXPLORER_INSTRUCTIONS,
        &agent_input(&compact_report, &[]),
    )?;
    let reviewer = client.call_agent(
        "Reviewer",
        REVIEWER_INSTRUCTIONS,
        &agent_input(&compact_report, &[("Explorer output", &explorer)]),
    )?;
    let risk_manager = client.call_agent(
        "Risk manager",
        RISK_MANAGER_INSTRUCTIONS,
        &agent_input(
            &compact_report,
            &[
                ("Explorer output", &explorer),
                ("Reviewer output", &reviewer),
            ],
        ),
    )?;
    let final_output = client.call_agent(
        "Output writer",
        OUTPUT_WRITER_INSTRUCTIONS,
        &agent_input(
            &compact_report,
            &[
                ("Explorer output", &explorer),
                ("Reviewer output", &reviewer),
                ("Risk manager output", &risk_manager),
            ],
        ),
    )?;

    Ok(AiWorkflowReport {
        explorer,
        reviewer,
        risk_manager,
        final_output,
    })
}

trait AiAgentClient {
    fn call_agent(&mut self, name: &str, instructions: &str, input: &str)
    -> Result<String, String>;
}

#[derive(Debug, Clone)]
struct OpenAiClient {
    http: Client,
    api_key: String,
    model: String,
    max_output_tokens: u32,
}

impl OpenAiClient {
    fn new(options: &AiOptions) -> Result<Self, String> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY is required when --ai is enabled".to_string())?;
        if api_key.trim().is_empty() {
            return Err("OPENAI_API_KEY is required when --ai is enabled".to_string());
        }

        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("betting-daily-agent/0.1")
            .build()
            .map_err(|error| format!("failed to build OpenAI HTTP client: {error}"))?;

        Ok(Self {
            http,
            api_key,
            model: options.model.clone(),
            max_output_tokens: options.max_output_tokens,
        })
    }
}

impl AiAgentClient for OpenAiClient {
    fn call_agent(
        &mut self,
        name: &str,
        instructions: &str,
        input: &str,
    ) -> Result<String, String> {
        let payload = json!({
            "model": self.model,
            "instructions": instructions,
            "input": input,
            "max_output_tokens": self.max_output_tokens,
            "store": false
        });

        let response = self
            .http
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .map_err(|error| format!("{name} agent OpenAI request failed: {error}"))?;

        let status = response.status();
        let body = response
            .text()
            .map_err(|error| format!("{name} agent response body read failed: {error}"))?;

        if !status.is_success() {
            return Err(format!("{name} agent OpenAI API returned {status}: {body}"));
        }

        let value = serde_json::from_str::<Value>(&body)
            .map_err(|error| format!("{name} agent JSON parse failed: {error}"))?;
        extract_output_text(&value)
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| format!("{name} agent returned no output text"))
    }
}

fn should_skip_ai_for_empty_no_bet_report(deterministic_report: &str) -> bool {
    deterministic_report.contains("Decision: NO BET")
        && deterministic_report.contains("No viable bets available")
}

fn extract_output_text(value: &Value) -> Option<String> {
    if let Some(output_text) = value.get("output_text").and_then(Value::as_str) {
        return Some(output_text.to_string());
    }

    let mut text_parts = Vec::new();
    for item in value.get("output")?.as_array()? {
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for content_item in content {
            if let Some(text) = content_item.get("text").and_then(Value::as_str) {
                text_parts.push(text.to_string());
            }
        }
    }

    if text_parts.is_empty() {
        None
    } else {
        Some(text_parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "final report",
        ]);

        let report = run_ai_workflow_with_client(WORKFLOW_FIXTURE, &mut client)
            .expect("mock workflow should succeed");

        assert_eq!(report.explorer, "explorer summary");
        assert_eq!(report.reviewer, "reviewer challenge");
        assert_eq!(report.risk_manager, "risk notes");
        assert_eq!(report.final_output, "final report");
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
}
