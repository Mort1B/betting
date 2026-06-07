use std::io::Read;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;

use super::models::{ClientContext, ContentId, ContentRequest, ContentResponse, Event, SportType};

const CONTENT_GET_URL: &str =
    "https://www.norsk-tipping.no/sport/oddsen/sportsbook/services/content/get";
const MAX_CONTENT_BODY_BYTES: u64 = 5_000_000;

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

        let response = self
            .client
            .post(CONTENT_GET_URL)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .header("poseidon", "oddsen")
            .json(&request)
            .send()
            .map_err(|error| format!("Norsk Tipping request failed: {error}"))?
            .error_for_status()
            .map_err(|error| format!("Norsk Tipping returned an HTTP error: {error}"))?;
        let body = read_limited_body(response, MAX_CONTENT_BODY_BYTES)?;

        parse_content_response(content_type, content_id, &body)
    }
}

fn read_limited_body(
    response: reqwest::blocking::Response,
    max_bytes: u64,
) -> Result<String, String> {
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

    if value
        .get("errorType")
        .and_then(Value::as_str)
        .is_some_and(|error_type| error_type == "CONTENT_NOT_FOUND")
    {
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
