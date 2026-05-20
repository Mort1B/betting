use reqwest::blocking::RequestBuilder;
use reqwest::header::HeaderMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CreditUsage {
    pub(super) remaining: Option<String>,
    pub(super) used: Option<String>,
    pub(super) last: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ApiResponse {
    pub(super) body: String,
    pub(super) credits: CreditUsage,
}

pub(super) fn send_request(request: RequestBuilder) -> Result<ApiResponse, String> {
    let response = request
        .send()
        .map_err(|error| format!("request failed: {error}"))?;
    let status = response.status();
    let credits = CreditUsage::from_headers(response.headers());
    if !status.is_success() {
        return Err(format!("HTTP {status}{}", credit_error_suffix(&credits)));
    }
    let body = response
        .text()
        .map_err(|error| format!("body read failed: {error}"))?;
    Ok(ApiResponse { body, credits })
}

impl CreditUsage {
    fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            remaining: header_value(headers, "x-requests-remaining"),
            used: header_value(headers, "x-requests-used"),
            last: header_value(headers, "x-requests-last"),
        }
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn credit_error_suffix(credits: &CreditUsage) -> String {
    let mut parts = Vec::new();
    if let Some(last) = credits.last.as_deref() {
        parts.push(format!("last {last}"));
    }
    if let Some(remaining) = credits.remaining.as_deref() {
        parts.push(format!("remaining {remaining}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("; API credits {}", parts.join(", "))
    }
}
