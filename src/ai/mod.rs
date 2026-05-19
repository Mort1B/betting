use std::env;
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{Value, json};

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

    let client = OpenAiClient::new(options)?;

    let explorer = client.call_agent(
        "Explorer",
        EXPLORER_INSTRUCTIONS,
        &format!("Deterministic betting report:\n\n{deterministic_report}"),
    )?;
    let reviewer = client.call_agent(
        "Reviewer",
        REVIEWER_INSTRUCTIONS,
        &format!(
            "Deterministic betting report:\n\n{deterministic_report}\n\nExplorer output:\n\n{explorer}"
        ),
    )?;
    let risk_manager = client.call_agent(
        "Risk manager",
        RISK_MANAGER_INSTRUCTIONS,
        &format!(
            "Deterministic betting report:\n\n{deterministic_report}\n\nExplorer output:\n\n{explorer}\n\nReviewer output:\n\n{reviewer}"
        ),
    )?;
    let final_output = client.call_agent(
        "Output writer",
        OUTPUT_WRITER_INSTRUCTIONS,
        &format!(
            "Deterministic betting report:\n\n{deterministic_report}\n\nExplorer output:\n\n{explorer}\n\nReviewer output:\n\n{reviewer}\n\nRisk manager output:\n\n{risk_manager}"
        ),
    )?;

    Ok(Some(AiWorkflowReport {
        explorer,
        reviewer,
        risk_manager,
        final_output,
    }))
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

    fn call_agent(&self, name: &str, instructions: &str, input: &str) -> Result<String, String> {
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

const EXPLORER_INSTRUCTIONS: &str = r#"You are the Explorer agent for a daily betting workflow.
Use the supplied deterministic report only. Identify the strongest probability, context, confidence, and research signals for the top candidates.
For every candidate, summarize supplied evidence for form, injuries/suspensions, lineups/rotation, motivation, schedule/travel pressure, weather/venue, market context, the learning note, research matches, and optional model/reference evidence.
Treat unknown football checklist items as missing evidence, not as positive or negative facts. Do not infer team news, motivation, injuries, odds, probabilities, sources, or results beyond the supplied report. Keep output concise."#;

const REVIEWER_INSTRUCTIONS: &str = r#"You are the Reviewer agent.
Challenge the Explorer and deterministic ranking. Look for overclaiming, weak football context evidence, stale or missing research, unresolved team news, underestimated form/injury/motivation/schedule risk, and cases where a bet is likely but not supported enough.
Check that the learning note is not overstated: insufficient history or no settled data must not become a confidence claim.
Return concise bullets with approve/question/reject style judgments for each top candidate.
Do not invent facts, do not add unsupplied football context, and do not recommend bets outside the supplied Norsk Tipping odds band."#;

const RISK_MANAGER_INSTRUCTIONS: &str = r#"You are the Risk Manager agent.
Identify downside risks, confidence concerns, missing data, and no-bet triggers. Treat gambling outcomes as uncertain and never imply a guaranteed win.
Downgrade or question candidates when injuries, suspensions, lineup, rotation, motivation, schedule, weather, venue, market context, or learning support is unresolved, negative, or insufficient in the supplied report.
Preserve deterministic fallback status and rejection reasons; do not turn a fallback candidate into a strict recommendation.
Return concise risk notes for each top candidate and say whether any candidate should be downgraded.
Use only supplied facts."#;

const OUTPUT_WRITER_INSTRUCTIONS: &str = r#"You are the Output Writer agent.
Write the final user-facing daily report using the deterministic report plus the Explorer, Reviewer, and Risk Manager outputs.
The output must include the top 5 candidates when available, preserving deterministic rank order. For each candidate include: sport/competition, event, market, selection, Norsk Tipping odds, probability/confidence basis, football context checklist summary, learning note, reference-market comparison only when supplied, main risks, strict rules status, and confidence score out of 100.
If the deterministic report says TOP 5 CANDIDATES, preserve those five ranked candidates and their fallback warnings instead of converting the report to NO BET.
If the deterministic report says NO BET because no candidates were supplied, output NO BET and explain why.
Keep unknown football context visible as unknown. Keep it practical, concise, and suitable for an iPhone notification/page. Do not invent facts."#;

#[cfg(test)]
mod tests {
    use super::*;

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
    fn ai_role_prompts_preserve_context_and_learning_constraints() {
        assert!(EXPLORER_INSTRUCTIONS.contains("learning note"));
        assert!(EXPLORER_INSTRUCTIONS.contains("unknown football checklist"));
        assert!(REVIEWER_INSTRUCTIONS.contains("insufficient history"));
        assert!(RISK_MANAGER_INSTRUCTIONS.contains("fallback status"));
        assert!(OUTPUT_WRITER_INSTRUCTIONS.contains("top 5 candidates"));
        assert!(OUTPUT_WRITER_INSTRUCTIONS.contains("learning note"));
        assert!(OUTPUT_WRITER_INSTRUCTIONS.contains("Do not invent facts"));
    }
}
