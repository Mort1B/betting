use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::env;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::analyze::{ResearchDigest, ResearchPage, analyze_text};
use super::reddit::{listing_pages, thread_search_pages};
use super::source::{ResearchOptions, ResearchSource, ResearchSourceKind};

const MAX_PARALLEL_FETCHES: usize = 4;
const MAX_RESEARCH_BODY_BYTES: u64 = 1_500_000;
const REDDIT_TOKEN_URL: &str = "https://www.reddit.com/api/v1/access_token";
const REDDIT_OAUTH_BASE_URL: &str = "https://oauth.reddit.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) betting-daily-agent/0.1";

#[derive(Debug, Clone)]
pub struct MarketResearchClient {
    http: Client,
    reddit_oauth: Option<RedditOAuthClient>,
    reddit_config_error: Option<String>,
}

impl MarketResearchClient {
    pub fn new() -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json,text/html;q=0.9,*/*;q=0.8"),
        );
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .http1_only()
            .build()
            .map_err(|error| format!("failed to build HTTP client: {error}"))?;

        let (reddit_oauth, reddit_config_error) = match RedditOAuthConfig::from_env() {
            Ok(Some(config)) => (Some(RedditOAuthClient::new(http.clone(), config)), None),
            Ok(None) => (None, None),
            Err(error) => (None, Some(error)),
        };

        Ok(Self {
            http,
            reddit_oauth,
            reddit_config_error,
        })
    }

    pub fn fetch(&self, sources: &[ResearchSource], options: &ResearchOptions) -> ResearchDigest {
        let selected = sources
            .iter()
            .take(options.max_pages)
            .cloned()
            .collect::<Vec<_>>();
        let mut indexed_pages = Vec::new();

        for (batch_index, batch) in selected.chunks(MAX_PARALLEL_FETCHES).enumerate() {
            let batch_start = batch_index * MAX_PARALLEL_FETCHES;
            thread::scope(|scope| {
                let handles = batch
                    .iter()
                    .enumerate()
                    .map(|(offset, source)| {
                        let client = self.clone();
                        let source = source.clone();
                        scope.spawn(move || {
                            (
                                batch_start + offset,
                                client.fetch_source_or_error(&source, options.max_items_per_source),
                            )
                        })
                    })
                    .collect::<Vec<_>>();

                for handle in handles {
                    indexed_pages.push(handle.join().expect("research fetch worker panicked"));
                }
            });
        }

        ResearchDigest {
            pages: flatten_ordered_pages(indexed_pages),
        }
    }

    fn fetch_source_or_error(
        &self,
        source: &ResearchSource,
        max_items: usize,
    ) -> Vec<ResearchPage> {
        match self.fetch_source(source, max_items) {
            Ok(source_pages) => source_pages,
            Err(error) => vec![source_error_page(source, error)],
        }
    }

    fn fetch_source(
        &self,
        source: &ResearchSource,
        max_items: usize,
    ) -> Result<Vec<ResearchPage>, String> {
        let body = match source.kind {
            ResearchSourceKind::Html => self.fetch_body(&source.url)?,
            ResearchSourceKind::RedditJson | ResearchSourceKind::RedditThreadSearch => {
                self.fetch_reddit_body(&source.url)?
            }
        };

        match source.kind {
            ResearchSourceKind::Html => Ok(vec![html_page(source, &body)]),
            ResearchSourceKind::RedditJson => listing_pages(source, &body, max_items),
            ResearchSourceKind::RedditThreadSearch => {
                thread_search_pages(source, &body, max_items, |url| self.fetch_reddit_body(url))
            }
        }
    }

    fn fetch_body(&self, url: &str) -> Result<String, String> {
        let response = self
            .http
            .get(url)
            .send()
            .map_err(|error| format!("{url}: {error}"))?;

        if !response.status().is_success() {
            return Err(format!("{url} returned {}", response.status()));
        }

        read_limited_body(response, url, MAX_RESEARCH_BODY_BYTES)
    }

    fn fetch_reddit_body(&self, url: &str) -> Result<String, String> {
        let Some(reddit_oauth) = &self.reddit_oauth else {
            return Err(self.reddit_config_error.clone().unwrap_or_else(|| {
                "Reddit API credentials not configured; skipping Reddit source".to_string()
            }));
        };

        reddit_oauth.fetch_body(url)
    }
}

fn read_limited_body(
    response: reqwest::blocking::Response,
    url: &str,
    max_bytes: u64,
) -> Result<String, String> {
    let mut limited = response.take(max_bytes + 1);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{url} body read failed: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("{url} exceeded {max_bytes} byte response limit"));
    }
    String::from_utf8(bytes).map_err(|error| format!("{url} body was not UTF-8: {error}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RedditOAuthConfig {
    client_id: String,
    client_secret: String,
    user_agent: String,
    token_url: String,
    oauth_base_url: String,
}

impl RedditOAuthConfig {
    fn from_env() -> Result<Option<Self>, String> {
        let client_id = optional_env("BETTING_REDDIT_CLIENT_ID");
        let client_secret = optional_env("BETTING_REDDIT_CLIENT_SECRET");
        match (client_id, client_secret) {
            (Some(client_id), Some(client_secret)) => Ok(Some(Self {
                client_id,
                client_secret,
                user_agent: optional_env("BETTING_REDDIT_USER_AGENT")
                    .unwrap_or_else(|| USER_AGENT.to_string()),
                token_url: optional_env("BETTING_REDDIT_TOKEN_URL")
                    .unwrap_or_else(|| REDDIT_TOKEN_URL.to_string()),
                oauth_base_url: optional_env("BETTING_REDDIT_OAUTH_BASE_URL")
                    .unwrap_or_else(|| REDDIT_OAUTH_BASE_URL.to_string()),
            })),
            (None, None) => Ok(None),
            _ => Err(
                "Reddit API credentials incomplete; configure both client ID and client secret"
                    .to_string(),
            ),
        }
    }
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Clone)]
struct RedditOAuthClient {
    http: Client,
    config: Arc<RedditOAuthConfig>,
    access_token: Arc<Mutex<Option<String>>>,
}

impl RedditOAuthClient {
    fn new(http: Client, config: RedditOAuthConfig) -> Self {
        Self {
            http,
            config: Arc::new(config),
            access_token: Arc::new(Mutex::new(None)),
        }
    }

    fn fetch_body(&self, url: &str) -> Result<String, String> {
        let token = self.access_token()?;
        let oauth_url = reddit_oauth_url(url, &self.config.oauth_base_url)?;
        let response = self
            .http
            .get(&oauth_url)
            .bearer_auth(token)
            .header("user-agent", self.config.user_agent.as_str())
            .send()
            .map_err(|error| self.sanitize_error(format!("{oauth_url}: {error}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = read_limited_body(response, &oauth_url, MAX_RESEARCH_BODY_BYTES)
                .unwrap_or_else(|error| error);
            return Err(self.sanitize_error(format!(
                "{oauth_url} returned {status}; body: {}",
                body_excerpt(&body)
            )));
        }

        read_limited_body(response, &oauth_url, MAX_RESEARCH_BODY_BYTES)
            .map_err(|error| self.sanitize_error(error))
    }

    fn access_token(&self) -> Result<String, String> {
        let mut access_token = self.access_token.lock().expect("token lock");
        if let Some(token) = access_token.clone() {
            return Ok(token);
        }

        let token = self.fetch_access_token()?;
        *access_token = Some(token.clone());
        Ok(token)
    }

    fn fetch_access_token(&self) -> Result<String, String> {
        let response = self
            .http
            .post(&self.config.token_url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .header("user-agent", self.config.user_agent.as_str())
            .form(&[("grant_type", "client_credentials")])
            .send()
            .map_err(|error| {
                self.sanitize_error(format!("Reddit token request failed: {error}"))
            })?;

        let status = response.status();
        let body = read_limited_body(response, &self.config.token_url, MAX_RESEARCH_BODY_BYTES)
            .map_err(|error| self.sanitize_error(error))?;
        if !status.is_success() {
            return Err(self.sanitize_error(format!(
                "Reddit token request returned {status}; body: {}",
                body_excerpt(&body)
            )));
        }

        let token = serde_json::from_str::<RedditTokenResponse>(&body).map_err(|error| {
            self.sanitize_error(format!("Reddit token JSON parse failed: {error}"))
        })?;
        if token.access_token.trim().is_empty() {
            return Err("Reddit token response did not include an access token".to_string());
        }

        Ok(token.access_token)
    }

    fn sanitize_error(&self, message: String) -> String {
        message
            .replace(&self.config.client_secret, "<redacted>")
            .replace(&self.config.client_id, "<redacted>")
    }
}

#[derive(Debug, Deserialize)]
struct RedditTokenResponse {
    access_token: String,
}

fn reddit_oauth_url(url: &str, oauth_base_url: &str) -> Result<String, String> {
    let parsed_url = reqwest::Url::parse(url)
        .map_err(|error| format!("invalid Reddit URL for OAuth fetch: {url}: {error}"))?;
    let host = parsed_url
        .host_str()
        .ok_or_else(|| format!("unsupported Reddit URL for OAuth fetch: {url}"))?;
    if parsed_url.scheme() != "https"
        || !matches!(
            host,
            "oauth.reddit.com"
                | "api.reddit.com"
                | "www.reddit.com"
                | "old.reddit.com"
                | "reddit.com"
        )
    {
        return Err(format!("unsupported Reddit URL for OAuth fetch: {url}"));
    }

    let mut oauth_url = reqwest::Url::parse(oauth_base_url)
        .map_err(|error| format!("invalid Reddit OAuth base URL: {oauth_base_url}: {error}"))?;
    oauth_url.set_path(parsed_url.path());
    oauth_url.set_query(parsed_url.query());
    Ok(oauth_url.to_string())
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

fn flatten_ordered_pages(mut indexed_pages: Vec<(usize, Vec<ResearchPage>)>) -> Vec<ResearchPage> {
    indexed_pages.sort_by_key(|(index, _)| *index);
    indexed_pages
        .into_iter()
        .flat_map(|(_, pages)| pages)
        .collect()
}

fn source_error_page(source: &ResearchSource, error: String) -> ResearchPage {
    ResearchPage::source_error(source.name.clone(), source.url.clone(), error)
}

fn html_page(source: &ResearchSource, body: &str) -> ResearchPage {
    let document = Html::parse_document(body);
    let title = parse_selector("title")
        .and_then(|selector| {
            document
                .select(&selector)
                .next()
                .map(|node| node.text().collect::<Vec<_>>().join(" "))
        })
        .unwrap_or_else(|| source.name.clone());
    let text = parse_selector("body")
        .and_then(|selector| {
            document
                .select(&selector)
                .next()
                .map(|node| node.text().collect::<Vec<_>>().join(" "))
        })
        .unwrap_or_default();
    let signals = analyze_text(&source.name, &format!("{title} {text}"));

    ResearchPage::new(
        source.name.clone(),
        source.url.clone(),
        title,
        text,
        signals,
        None,
    )
}

fn parse_selector(selector: &str) -> Option<Selector> {
    Selector::parse(selector).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn flattens_parallel_results_by_source_order() {
        let pages = flatten_ordered_pages(vec![
            (1, vec![page("second-a"), page("second-b")]),
            (0, vec![page("first")]),
            (2, vec![page("third")]),
        ]);

        let titles = pages.into_iter().map(|page| page.title).collect::<Vec<_>>();
        assert_eq!(titles, vec!["first", "second-a", "second-b", "third"]);
    }

    #[test]
    fn source_error_page_preserves_source_identity() {
        let source = ResearchSource {
            name: "blocked".to_string(),
            kind: ResearchSourceKind::Html,
            url: "https://example.test".to_string(),
        };

        let page = source_error_page(&source, "returned 403".to_string());

        assert_eq!(page.source_name, "blocked");
        assert_eq!(page.url, "https://example.test");
        assert_eq!(page.error.as_deref(), Some("returned 403"));
    }

    #[test]
    fn fetches_parallel_sources_with_stable_order_and_errors() {
        let slow = MockServer::spawn(
            "200 OK",
            "<html><head><title>Slow Source</title></head><body>slow value</body></html>",
            Duration::from_millis(100),
        );
        let fast = MockServer::spawn(
            "200 OK",
            "<html><head><title>Fast Source</title></head><body>fast value</body></html>",
            Duration::ZERO,
        );
        let failed = MockServer::spawn(
            "500 Internal Server Error",
            "failed",
            Duration::from_millis(10),
        );
        let sources = vec![
            source("slow", &slow.url),
            source("fast", &fast.url),
            source("failed", &failed.url),
        ];
        let options = ResearchOptions {
            source_path: None,
            max_pages: 10,
            max_items_per_source: 10,
        };

        let digest = MarketResearchClient::new()
            .expect("client")
            .fetch(&sources, &options);

        slow.join();
        fast.join();
        failed.join();
        assert_eq!(digest.pages.len(), 3);
        assert_eq!(digest.pages[0].source_name, "slow");
        assert_eq!(digest.pages[0].title, "Slow Source");
        assert_eq!(digest.pages[1].source_name, "fast");
        assert_eq!(digest.pages[1].title, "Fast Source");
        assert_eq!(digest.pages[2].source_name, "failed");
        assert!(
            digest.pages[2]
                .error
                .as_deref()
                .is_some_and(|error| error.contains("500"))
        );
    }

    #[test]
    fn reddit_oauth_url_rewrites_known_reddit_hosts() {
        assert_eq!(
            reddit_oauth_url(
                "https://api.reddit.com/r/test/search?q=daily",
                "http://127.0.0.1:12345"
            )
            .expect("api URL rewrites"),
            "http://127.0.0.1:12345/r/test/search?q=daily"
        );
        assert_eq!(
            reddit_oauth_url(
                "https://www.reddit.com/r/test/comments/post.json?limit=5",
                "https://oauth.reddit.com"
            )
            .expect("www URL rewrites"),
            "https://oauth.reddit.com/r/test/comments/post.json?limit=5"
        );

        assert!(
            reddit_oauth_url("https://example.test/r/test", "https://oauth.reddit.com").is_err()
        );
        assert!(
            reddit_oauth_url("https://reddit.com.evil/r/test", "https://oauth.reddit.com").is_err()
        );
    }

    #[test]
    fn reddit_source_without_credentials_returns_clean_source_error() {
        let client = MarketResearchClient {
            http: test_http_client(),
            reddit_oauth: None,
            reddit_config_error: None,
        };
        let source = reddit_source("https://api.reddit.com/r/test/top.json");

        let pages = client.fetch_source_or_error(&source, 10);

        assert_eq!(pages.len(), 1);
        assert_eq!(
            pages[0].error.as_deref(),
            Some("Reddit API credentials not configured; skipping Reddit source")
        );
    }

    #[test]
    fn reddit_oauth_fetches_listing_with_token() {
        let token_server = MockServer::spawn(
            "200 OK",
            r#"{"access_token":"test-token","token_type":"bearer","expires_in":3600}"#,
            Duration::ZERO,
        );
        let api_server = MockServer::spawn(
            "200 OK",
            r#"{"data":{"children":[{"kind":"t3","data":{"title":"Daily Picks Thread","selftext":"Rosenborg value","permalink":"/r/test/comments/1/post/"}}]}}"#,
            Duration::ZERO,
        );
        let client = reddit_test_client(&token_server.url, &api_server.url);
        let source = reddit_source("https://api.reddit.com/r/test/top.json");

        let pages = client.fetch_source_or_error(&source, 10);

        let token_request = token_server.join();
        let api_request = api_server.join();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Daily Picks Thread");
        assert!(pages[0].text.contains("Rosenborg value"));
        assert!(pages[0].error.is_none());
        assert!(
            token_request
                .to_ascii_lowercase()
                .contains("authorization: basic ")
        );
        assert!(
            api_request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-token")
        );
    }

    #[test]
    fn reddit_token_errors_redact_credentials() {
        let token_server = MockServer::spawn(
            "401 Unauthorized",
            "client-id secret-value invalid",
            Duration::ZERO,
        );
        let client = reddit_test_client(&token_server.url, "http://127.0.0.1:9");
        let source = reddit_source("https://api.reddit.com/r/test/top.json");

        let pages = client.fetch_source_or_error(&source, 10);

        token_server.join();
        let error = pages[0].error.as_deref().expect("source error");
        assert!(error.contains("<redacted>"));
        assert!(!error.contains("client-id"));
        assert!(!error.contains("secret-value"));
    }

    fn page(title: &str) -> ResearchPage {
        ResearchPage::new(
            title.to_string(),
            format!("https://example.test/{title}"),
            title.to_string(),
            String::new(),
            Vec::new(),
            None,
        )
    }

    fn source(name: &str, url: &str) -> ResearchSource {
        ResearchSource {
            name: name.to_string(),
            kind: ResearchSourceKind::Html,
            url: url.to_string(),
        }
    }

    fn reddit_source(url: &str) -> ResearchSource {
        ResearchSource {
            name: "reddit daily".to_string(),
            kind: ResearchSourceKind::RedditJson,
            url: url.to_string(),
        }
    }

    fn reddit_test_client(token_url: &str, oauth_base_url: &str) -> MarketResearchClient {
        let http = test_http_client();
        MarketResearchClient {
            http: http.clone(),
            reddit_oauth: Some(RedditOAuthClient::new(
                http,
                RedditOAuthConfig {
                    client_id: "client-id".to_string(),
                    client_secret: "secret-value".to_string(),
                    user_agent: USER_AGENT.to_string(),
                    token_url: token_url.to_string(),
                    oauth_base_url: oauth_base_url.to_string(),
                },
            )),
            reddit_config_error: None,
        }
    }

    fn test_http_client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent(USER_AGENT)
            .build()
            .expect("test client")
    }

    struct MockServer {
        url: String,
        handle: thread::JoinHandle<String>,
    }

    impl MockServer {
        fn spawn(status: &'static str, body: &'static str, delay: Duration) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
            let url = format!("http://{}", listener.local_addr().expect("mock addr"));
            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let bytes_read = stream.read(&mut buffer).expect("read request");
                let request = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                thread::sleep(delay);
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\ncontent-length: {}\r\ncontent-type: text/html\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                )
                .expect("write response");
                request
            });

            Self { url, handle }
        }

        fn join(self) -> String {
            self.handle.join().expect("mock server finished")
        }
    }
}
