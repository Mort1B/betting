use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

use super::models::{ClientContext, ContentId, ContentRequest, ContentResponse, Event, SportType};

const CONTENT_GET_URL: &str =
    "https://www.norsk-tipping.no/sport/oddsen/sportsbook/services/content/get";

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

        self.client
            .post(CONTENT_GET_URL)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .header("poseidon", "oddsen")
            .json(&request)
            .send()
            .map_err(|error| format!("Norsk Tipping request failed: {error}"))?
            .error_for_status()
            .map_err(|error| format!("Norsk Tipping returned an HTTP error: {error}"))?
            .json::<T>()
            .map_err(|error| format!("failed to parse Norsk Tipping response: {error}"))
    }
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
}
