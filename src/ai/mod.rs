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
            max_output_tokens: 3_500,
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
    validate_final_output(&compact_report, &final_output)?;

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
        if let Some(reason) = incomplete_response_reason(&value) {
            return Err(format!("{name} agent returned incomplete output: {reason}"));
        }
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

fn incomplete_response_reason(value: &Value) -> Option<String> {
    if value.get("status").and_then(Value::as_str) != Some("incomplete") {
        return None;
    }

    Some(
        value
            .pointer("/incomplete_details/reason")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
    )
}

fn validate_final_output(compact_report: &str, final_output: &str) -> Result<(), String> {
    let expected_headings = ranked_candidate_headings(compact_report);
    if expected_headings.is_empty() {
        return Ok(());
    }

    let found = ranked_candidate_count(final_output);
    if found < expected_headings.len() {
        return Err(format!(
            "Output writer omitted ranked candidates: expected {}, found {found}",
            expected_headings.len()
        ));
    }
    for heading in &expected_headings {
        if !final_output.lines().any(|line| line.trim() == heading) {
            return Err(format!(
                "Output writer omitted ranked candidate heading: {heading}"
            ));
        }
    }
    for line in required_summary_lines(compact_report) {
        if !final_output.contains(&line) {
            return Err(format!("Output writer omitted summary line: {line}"));
        }
    }

    Ok(())
}

fn ranked_candidate_headings(report: &str) -> Vec<String> {
    report
        .lines()
        .map(str::trim)
        .filter(|line| is_ranked_candidate_heading(line))
        .map(str::to_string)
        .collect()
}

fn ranked_candidate_count(report: &str) -> usize {
    report
        .lines()
        .map(str::trim_start)
        .filter(|line| is_ranked_candidate_heading(line))
        .count()
}

fn is_ranked_candidate_heading(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('#') else {
        return false;
    };
    rest.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn required_summary_lines(report: &str) -> Vec<String> {
    report
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.starts_with("Football data provider:")
                || line.starts_with("Football data provider note:")
                || line.starts_with("Reference provider:")
                || line.starts_with("Reference provider note:")
        })
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests;
