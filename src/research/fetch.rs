use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use scraper::{Html, Selector};
use std::process::Command;
use std::thread;
use std::time::Duration;

use super::analyze::{ResearchDigest, ResearchPage, analyze_text};
use super::reddit::{listing_pages, thread_search_pages};
use super::source::{ResearchOptions, ResearchSource, ResearchSourceKind};

const MAX_PARALLEL_FETCHES: usize = 4;
const USER_AGENT: &str = "betting-daily-agent/0.1 by local-user";

#[derive(Debug, Clone)]
pub struct MarketResearchClient {
    http: Client,
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

        Ok(Self { http })
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

        response
            .text()
            .map_err(|error| format!("{url} body read failed: {error}"))
    }

    fn fetch_reddit_body(&self, url: &str) -> Result<String, String> {
        match curl_fetch_body(url) {
            Ok(body) => Ok(body),
            Err(curl_error) => self
                .fetch_body(url)
                .map_err(|error| format!("curl failed: {curl_error}; reqwest failed: {error}")),
        }
    }
}

fn curl_fetch_body(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args([
            "-L",
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            "20",
            "-A",
            USER_AGENT,
            url,
        ])
        .output()
        .map_err(|error| format!("failed to start curl: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl exited {}: {}", output.status, stderr.trim()));
    }

    String::from_utf8(output.stdout).map_err(|error| format!("curl output was not UTF-8: {error}"))
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

    struct MockServer {
        url: String,
        handle: thread::JoinHandle<()>,
    }

    impl MockServer {
        fn spawn(status: &'static str, body: &'static str, delay: Duration) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
            let url = format!("http://{}", listener.local_addr().expect("mock addr"));
            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                thread::sleep(delay);
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\ncontent-length: {}\r\ncontent-type: text/html\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                )
                .expect("write response");
            });

            Self { url, handle }
        }

        fn join(self) {
            self.handle.join().expect("mock server finished");
        }
    }
}
