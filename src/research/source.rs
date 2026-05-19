use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub struct ResearchOptions {
    pub source_path: Option<String>,
    pub max_pages: usize,
    pub max_items_per_source: usize,
}

impl Default for ResearchOptions {
    fn default() -> Self {
        Self {
            source_path: None,
            max_pages: 10,
            max_items_per_source: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResearchSource {
    pub name: String,
    pub kind: ResearchSourceKind,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchSourceKind {
    Html,
    RedditJson,
    RedditThreadSearch,
}

impl ResearchSourceKind {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_lowercase().as_str() {
            "html" | "web" | "page" => Ok(Self::Html),
            "reddit_json" | "reddit" => Ok(Self::RedditJson),
            "reddit_thread_search" | "reddit_daily_thread" => Ok(Self::RedditThreadSearch),
            other => Err(format!("unsupported research source kind: {other}")),
        }
    }
}

pub fn load_sources(options: &ResearchOptions) -> Result<Vec<ResearchSource>, String> {
    let Some(path) = &options.source_path else {
        return Ok(Vec::new());
    };

    let content = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    let mut sources = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let fields = trimmed.split('|').map(str::trim).collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(format!(
                "line {}: expected name|kind|url, got {trimmed}",
                index + 1
            ));
        }

        sources.push(ResearchSource {
            name: fields[0].to_string(),
            kind: ResearchSourceKind::parse(fields[1])?,
            url: fields[2].to_string(),
        });
    }

    Ok(sources.into_iter().take(options.max_pages).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kind_aliases() {
        assert_eq!(
            ResearchSourceKind::parse("reddit").expect("valid"),
            ResearchSourceKind::RedditJson
        );
        assert_eq!(
            ResearchSourceKind::parse("reddit_daily_thread").expect("valid"),
            ResearchSourceKind::RedditThreadSearch
        );
        assert_eq!(
            ResearchSourceKind::parse("web").expect("valid"),
            ResearchSourceKind::Html
        );
    }
}
