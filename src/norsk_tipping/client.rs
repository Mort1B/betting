use std::error::Error;
use std::io::Read;
use std::thread;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde::Deserialize;
use serde_json::Value;

use super::models::{ClientContext, ContentId, ContentRequest, ContentResponse, Event, SportType};

const CONTENT_GET_URL: &str =
    "https://www.norsk-tipping.no/sport/oddsen/sportsbook/services/content/get";
const MAX_CONTENT_BODY_BYTES: u64 = 5_000_000;
const MAX_FETCH_ATTEMPTS: usize = 3;

pub(crate) struct LiveOddsClient {
    client: Client,
}

impl LiveOddsClient {
    pub(crate) fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(25))
            .user_agent("betting-agent/0.1")
            .build()
            .map_err(|error| format!("failed to create HTTP client: {error}"))?;
        Ok(Self { client })
    }

    pub(crate) fn fetch_sport_types(&self, compact_date: &str) -> Result<Vec<SportType>, String> {
        let content_id = format!("{compact_date}0000/{compact_date}2359/");
        let response: ContentResponse<SportType> =
            self.fetch_content("sportTypeByDate", &content_id)?;
        Ok(response.data)
    }

    pub(crate) fn fetch_events(
        &self,
        sport_id: &str,
        compact_date: &str,
        events_per_sport: usize,
    ) -> Result<Vec<Event>, String> {
        let content_id = format!("{sport_id}/{compact_date}/0/{events_per_sport}/D");
        let response: ContentResponse<Event> =
            self.fetch_content("eventListBySportTypeDay", &content_id)?;
        Ok(response.data)
    }

    fn fetch_content<T>(&self, content_type: &str, content_id: &str) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        let request = ContentRequest {
            content_id: ContentId {
                kind: content_type,
                id: content_id,
            },
            client_context: ClientContext {
                language: "NO",
                ip_address: "0.0.0.0",
            },
        };

        let mut attempt_errors = Vec::new();
        for attempt in 1..=MAX_FETCH_ATTEMPTS {
            match self.fetch_content_body(content_type, content_id, &request) {
                Ok(body) => match parse_content_response(content_type, content_id, &body) {
                    Ok(response) => return Ok(response),
                    Err(error) if is_retryable_content_error(&error) => {
                        attempt_errors.push(format!("attempt {attempt}: {error}"));
                    }
                    Err(error) => return Err(error),
                },
                Err(error) => {
                    let retryable = error.retryable;
                    attempt_errors.push(format!("attempt {attempt}: {}", error.message));
                    if !retryable {
                        break;
                    }
                }
            }

            if attempt < MAX_FETCH_ATTEMPTS {
                thread::sleep(retry_delay(attempt));
            }
        }

        let last_error = attempt_errors
            .last()
            .cloned()
            .unwrap_or_else(|| "no attempt details available".to_string());
        Err(format!(
            "Norsk Tipping request failed after {MAX_FETCH_ATTEMPTS} attempt(s); {last_error}"
        ))
    }

    fn fetch_content_body(
        &self,
        content_type: &str,
        content_id: &str,
        request: &ContentRequest<'_>,
    ) -> Result<String, FetchAttemptError> {
        let response = self
            .client
            .post(CONTENT_GET_URL)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .header("poseidon", "oddsen")
            .json(request)
            .send()
            .map_err(|error| {
                let retryable = error.is_timeout() || error.is_connect() || error.is_request();
                FetchAttemptError {
                    retryable,
                    message: format!(
                        "Norsk Tipping request failed for {content_type} {content_id}: {}",
                        error_chain(&error)
                    ),
                }
            })?;
        let status = response.status();
        let body = read_limited_body(response, MAX_CONTENT_BODY_BYTES).map_err(|error| {
            FetchAttemptError {
                retryable: true,
                message: error,
            }
        })?;
        if !status.is_success() {
            return Err(FetchAttemptError {
                retryable: is_retryable_status(status),
                message: format!(
                    "Norsk Tipping returned HTTP {status} for {content_type} {content_id}; body: {}",
                    body_excerpt(&body)
                ),
            });
        }

        Ok(body)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FetchAttemptError {
    message: String,
    retryable: bool,
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis((attempt as u64) * 300)
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_content_error(error: &str) -> bool {
    error.contains("Norsk Tipping returned INTERNAL_ERROR")
}

fn error_chain(error: &reqwest::Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(error) = source {
        parts.push(error.to_string());
        source = error.source();
    }
    parts.join(": ")
}

fn read_limited_body(response: Response, max_bytes: u64) -> Result<String, String> {
    let mut limited = response.take(max_bytes + 1);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read Norsk Tipping response: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!(
            "Norsk Tipping response exceeded {max_bytes} byte limit"
        ));
    }
    String::from_utf8(bytes)
        .map_err(|error| format!("Norsk Tipping response was not UTF-8: {error}"))
}

fn parse_content_response<T>(content_type: &str, content_id: &str, body: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let value = serde_json::from_str::<Value>(body).map_err(|error| {
        format!(
            "failed to parse Norsk Tipping response JSON for {content_type} {content_id}: {error}; body: {}",
            body_excerpt(body)
        )
    })?;

    if let Some(error_type) = value.get("errorType").and_then(Value::as_str) {
        if error_type != "CONTENT_NOT_FOUND" {
            return Err(format!(
                "Norsk Tipping returned {error_type} for {content_type} {content_id}; body: {}",
                body_excerpt(body)
            ));
        }
        return serde_json::from_value(Value::Object(
            [("data".to_string(), Value::Array(Vec::new()))]
                .into_iter()
                .collect(),
        ))
        .map_err(|error| {
            format!("failed to build empty Norsk Tipping response for {content_type}: {error}")
        });
    }

    serde_json::from_value::<T>(value).map_err(|error| {
        format!(
            "failed to decode Norsk Tipping response for {content_type} {content_id}: {error}; body: {}",
            body_excerpt(body)
        )
    })
}

fn body_excerpt(body: &str) -> String {
    const MAX_EXCERPT_CHARS: usize = 300;
    let trimmed = body.trim();
    if trimmed.chars().count() <= MAX_EXCERPT_CHARS {
        return trimmed.to_string();
    }
    let excerpt = trimmed.chars().take(MAX_EXCERPT_CHARS).collect::<String>();
    format!("{excerpt}...")
}

pub(crate) fn compact_date(date: &str) -> Result<String, String> {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(format!("date must use YYYY-MM-DD, got {date}"));
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return Err(format!("date must use YYYY-MM-DD, got {date}"));
    }
    Ok(date.replace('-', ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compacts_iso_date_for_norsk_tipping_content_ids() {
        assert_eq!(compact_date("2026-05-16").expect("valid date"), "20260516");
        assert!(compact_date("20260516").is_err());
    }

    #[test]
    fn content_not_found_decodes_as_empty_response() {
        let response: ContentResponse<Event> = parse_content_response(
            "eventListBySportTypeDay",
            "FBL/20260519/0/35/D",
            r#"{"errorType":"CONTENT_NOT_FOUND","data":["eventListBySportTypeDay","FBL/20260519/0/35/D","ContentServiceImpl.get()"]}"#,
        )
        .expect("content not found should be an empty board");

        assert!(response.data.is_empty());
    }

    #[test]
    fn internal_error_response_is_not_treated_as_empty_board() {
        let error = parse_content_response::<ContentResponse<Event>>(
            "sportTypeByDate",
            "202606070000/202606072359/",
            r#"{"errorType":"INTERNAL_ERROR","data":[]}"#,
        )
        .expect_err("internal error should be surfaced");

        assert!(error.contains("Norsk Tipping returned INTERNAL_ERROR"));
        assert!(error.contains("sportTypeByDate 202606070000/202606072359/"));
    }

    #[test]
    fn decode_errors_include_response_excerpt() {
        let error = parse_content_response::<ContentResponse<Event>>(
            "eventListBySportTypeDay",
            "FBL/20260519/0/35/D",
            r#"{"data":["unexpected"]}"#,
        )
        .expect_err("invalid data should fail");

        assert!(error.contains("eventListBySportTypeDay FBL/20260519/0/35/D"));
        assert!(error.contains(r#""unexpected""#));
    }
}
