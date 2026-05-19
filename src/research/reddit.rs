use serde::Deserialize;

use super::analyze::{ResearchPage, analyze_text};
use super::source::ResearchSource;

pub(super) fn listing_pages(
    source: &ResearchSource,
    body: &str,
    max_items: usize,
) -> Result<Vec<ResearchPage>, String> {
    let listing = parse_listing(&source.url, body)?;
    let mut pages = Vec::new();

    for child in listing.data.children.into_iter().take(max_items) {
        let title = child
            .data
            .title
            .unwrap_or_else(|| "Reddit post".to_string());
        let text = format!(
            "{}\n{}\n{}",
            title,
            child.data.selftext.unwrap_or_default(),
            child.data.url.unwrap_or_default()
        );
        let signals = analyze_text(&source.name, &text);
        let url = child
            .data
            .permalink
            .map(|path| reddit_url(&path))
            .unwrap_or_else(|| source.url.clone());
        pages.push(ResearchPage::new(
            source.name.clone(),
            url,
            title,
            text,
            signals,
            None,
        ));
    }

    Ok(pages)
}

pub(super) fn thread_search_pages<F>(
    source: &ResearchSource,
    body: &str,
    max_items: usize,
    fetch_body: F,
) -> Result<Vec<ResearchPage>, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    let listing = parse_listing(&source.url, body)?;
    let Some(thread) = select_daily_thread(&listing) else {
        return Ok(Vec::new());
    };
    let Some(permalink) = thread.permalink.as_deref() else {
        return Err("daily picks thread did not include a permalink".to_string());
    };
    let thread_url = reddit_comments_url(permalink, max_items);
    let comments_body = fetch_body(&thread_url)?;

    comment_pages(
        source,
        &thread_url,
        thread.title.as_deref(),
        &comments_body,
        max_items,
    )
}

fn parse_listing(url: &str, body: &str) -> Result<RedditListing, String> {
    serde_json::from_str::<RedditListing>(body)
        .map_err(|error| format!("{url} JSON parse failed: {error}"))
}

fn select_daily_thread(listing: &RedditListing) -> Option<&RedditItem> {
    let mut first_match = None;
    let mut first_with_comments = None;
    for child in &listing.data.children {
        let title = child.data.title.as_deref()?;
        if !is_daily_pick_thread(title) {
            continue;
        }
        first_match.get_or_insert(&child.data);
        let comments = child.data.num_comments.unwrap_or(0);
        if comments >= 5 {
            return Some(&child.data);
        }
        if comments > 0 {
            first_with_comments.get_or_insert(&child.data);
        }
    }
    first_with_comments.or(first_match)
}

fn is_daily_pick_thread(title: &str) -> bool {
    let title = title.to_lowercase();
    title.contains("daily picks thread")
        || title.contains("reddit daily picks")
        || title.contains("pick of the day")
        || title.contains("betting and picks daily discussion")
}

fn comment_pages(
    source: &ResearchSource,
    thread_url: &str,
    thread_title: Option<&str>,
    body: &str,
    max_items: usize,
) -> Result<Vec<ResearchPage>, String> {
    let listings = serde_json::from_str::<Vec<RedditListing>>(body)
        .map_err(|error| format!("{thread_url} JSON parse failed: {error}"))?;
    let comments = listings
        .get(1)
        .ok_or_else(|| format!("{thread_url} did not include comment listing"))?;
    let mut pages = Vec::new();
    let thread_title = thread_title.unwrap_or("Reddit daily picks thread");

    for child in &comments.data.children {
        if child.kind.as_deref() != Some("t1") {
            continue;
        }
        let Some(body) = child.data.body.as_deref().map(str::trim) else {
            continue;
        };
        if body.is_empty() || matches!(body, "[deleted]" | "[removed]") {
            continue;
        }

        let author = child.data.author.as_deref().unwrap_or("unknown");
        let score = child.data.score.unwrap_or(0);
        let title = format!("{thread_title} | comment by {author} | score {score}");
        let text = format!("{thread_title}\n{body}");
        let url = child
            .data
            .permalink
            .as_deref()
            .map(reddit_url)
            .unwrap_or_else(|| thread_url.to_string());
        let signals = analyze_text(&source.name, &text);
        pages.push(ResearchPage::new(
            source.name.clone(),
            url,
            title,
            text,
            signals,
            None,
        ));
        if pages.len() >= max_items {
            break;
        }
    }

    Ok(pages)
}

fn reddit_url(path: &str) -> String {
    if path.starts_with("http") {
        path.to_string()
    } else {
        format!("https://www.reddit.com{path}")
    }
}

fn reddit_comments_url(permalink: &str, limit: usize) -> String {
    let path = permalink
        .strip_prefix("https://www.reddit.com")
        .or_else(|| permalink.strip_prefix("https://reddit.com"))
        .or_else(|| permalink.strip_prefix("https://api.reddit.com"))
        .unwrap_or(permalink);
    format!(
        "https://api.reddit.com{}?sort=confidence&limit={}",
        path.trim_end_matches('/'),
        limit
    )
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
    kind: Option<String>,
    data: RedditItem,
}

#[derive(Debug, Deserialize)]
struct RedditItem {
    title: Option<String>,
    selftext: Option<String>,
    permalink: Option<String>,
    url: Option<String>,
    body: Option<String>,
    author: Option<String>,
    score: Option<i64>,
    num_comments: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::source::ResearchSourceKind;

    #[test]
    fn listing_pages_parse_reddit_listing_posts() {
        let source = source("https://www.reddit.com/r/soccerbetting/top.json");
        let pages = listing_pages(
            &source,
            r#"{"data":{"children":[{"kind":"t3","data":{"title":"Daily Picks Thread","selftext":"Rosenborg value","url":"https://example.test","permalink":"/r/test/comments/1/post/"}}]}}"#,
            10,
        )
        .expect("listing parses");

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Daily Picks Thread");
        assert!(pages[0].text.contains("Rosenborg value"));
        assert_eq!(
            pages[0].url,
            "https://www.reddit.com/r/test/comments/1/post/"
        );
    }

    #[test]
    fn daily_thread_selection_prefers_threads_with_comments() {
        let listing = parse_listing(
            "https://example.test",
            r#"{"data":{"children":[
              {"kind":"t3","data":{"title":"Daily Picks Thread - Wednesday","num_comments":0,"permalink":"/r/test/comments/new/"}},
              {"kind":"t3","data":{"title":"Daily Picks Thread - Tuesday","num_comments":33,"permalink":"/r/test/comments/old/"}}
            ]}}"#,
        )
        .expect("listing parses");

        let selected = select_daily_thread(&listing).expect("thread selected");
        assert_eq!(selected.permalink.as_deref(), Some("/r/test/comments/old/"));
    }

    #[test]
    fn daily_thread_selection_skips_low_comment_fresh_thread() {
        let listing = parse_listing(
            "https://example.test",
            r#"{"data":{"children":[
              {"kind":"t3","data":{"title":"Daily Picks Thread - Wednesday","num_comments":1,"permalink":"/r/test/comments/new/"}},
              {"kind":"t3","data":{"title":"Daily Picks Thread - Tuesday","num_comments":33,"permalink":"/r/test/comments/old/"}}
            ]}}"#,
        )
        .expect("listing parses");

        let selected = select_daily_thread(&listing).expect("thread selected");
        assert_eq!(selected.permalink.as_deref(), Some("/r/test/comments/old/"));
    }

    #[test]
    fn daily_thread_selection_returns_none_without_matching_title() {
        let listing = parse_listing(
            "https://example.test",
            r#"{"data":{"children":[
              {"kind":"t3","data":{"title":"General discussion","num_comments":8,"permalink":"/r/test/comments/general/"}}
            ]}}"#,
        )
        .expect("listing parses");

        assert!(select_daily_thread(&listing).is_none());
    }

    #[test]
    fn comment_url_uses_reddit_api_endpoint() {
        assert_eq!(
            reddit_comments_url("/r/test/comments/post/daily_picks_thread/", 5),
            "https://api.reddit.com/r/test/comments/post/daily_picks_thread?sort=confidence&limit=5"
        );
    }

    #[test]
    fn comment_pages_parse_daily_thread_comments() {
        let source = source("https://www.reddit.com/r/soccerbetting/search.json");
        let pages = comment_pages(
            &source,
            "https://www.reddit.com/r/test/comments/post.json",
            Some("Daily Picks Thread"),
            r#"[{"data":{"children":[]}},{"data":{"children":[
              {"kind":"t1","data":{"author":"capper","score":7,"body":"Rosenborg - Brann over 1.5, good form","permalink":"/r/test/comments/post/comment/"}},
              {"kind":"more","data":{}}
            ]}}]"#,
            10,
        )
        .expect("comments parse");

        assert_eq!(pages.len(), 1);
        assert!(pages[0].title.contains("capper"));
        assert!(pages[0].text.contains("good form"));
    }

    fn source(url: &str) -> ResearchSource {
        ResearchSource {
            name: "reddit daily".to_string(),
            kind: ResearchSourceKind::RedditThreadSearch,
            url: url.to_string(),
        }
    }
}
