use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::time::Duration;

use super::analyze::{ResearchDigest, ResearchPage, analyze_text};
use super::source::{ResearchOptions, ResearchSource, ResearchSourceKind};

#[derive(Debug, Clone)]
pub struct MarketResearchClient {
    http: Client,
}

impl MarketResearchClient {
    pub fn new() -> Result<Self, String> {
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("betting-daily-agent/0.1 by local-user")
            .build()
            .map_err(|error| format!("failed to build HTTP client: {error}"))?;

        Ok(Self { http })
    }

    pub fn fetch(&self, sources: &[ResearchSource], options: &ResearchOptions) -> ResearchDigest {
        let mut pages = Vec::new();

        for source in sources.iter().take(options.max_pages) {
            match self.fetch_source(source, options.max_items_per_source) {
                Ok(mut source_pages) => pages.append(&mut source_pages),
                Err(error) => pages.push(ResearchPage {
                    source_name: source.name.clone(),
                    url: source.url.clone(),
                    title: source.name.clone(),
                    text: String::new(),
                    signals: Vec::new(),
                    error: Some(error),
                }),
            }
        }

        ResearchDigest { pages }
    }

    fn fetch_source(
        &self,
        source: &ResearchSource,
        max_items: usize,
    ) -> Result<Vec<ResearchPage>, String> {
        let response = self
            .http
            .get(&source.url)
            .send()
            .map_err(|error| format!("{}: {error}", source.url))?;

        if !response.status().is_success() {
            return Err(format!("{} returned {}", source.url, response.status()));
        }

        let body = response
            .text()
            .map_err(|error| format!("{} body read failed: {error}", source.url))?;

        match source.kind {
            ResearchSourceKind::Html => Ok(vec![html_page(source, &body)]),
            ResearchSourceKind::RedditJson => reddit_pages(source, &body, max_items),
        }
    }
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

    ResearchPage {
        source_name: source.name.clone(),
        url: source.url.clone(),
        title,
        text,
        signals,
        error: None,
    }
}

fn reddit_pages(
    source: &ResearchSource,
    body: &str,
    max_items: usize,
) -> Result<Vec<ResearchPage>, String> {
    let listing = serde_json::from_str::<RedditListing>(body)
        .map_err(|error| format!("{} JSON parse failed: {error}", source.url))?;
    let mut pages = Vec::new();

    for child in listing.data.children.into_iter().take(max_items) {
        let text = format!(
            "{}\n{}\n{}",
            child.data.title,
            child.data.selftext.unwrap_or_default(),
            child.data.url.unwrap_or_default()
        );
        let signals = analyze_text(&source.name, &text);
        pages.push(ResearchPage {
            source_name: source.name.clone(),
            url: child
                .data
                .permalink
                .map(|path| format!("https://www.reddit.com{path}"))
                .unwrap_or_else(|| source.url.clone()),
            title: child.data.title,
            text,
            signals,
            error: None,
        });
    }

    Ok(pages)
}

fn parse_selector(selector: &str) -> Option<Selector> {
    Selector::parse(selector).ok()
}

#[derive(Debug, Deserialize)]
struct RedditListing {
    data: RedditListingData,
}

#[derive(Debug, Deserialize)]
struct RedditListingData {
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    title: String,
    selftext: Option<String>,
    permalink: Option<String>,
    url: Option<String>,
}
