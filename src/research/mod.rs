mod analyze;
mod fetch;
mod reddit;
mod source;

pub use analyze::{ResearchDigest, assess_candidate_research};

#[cfg(test)]
pub use analyze::{ResearchPage, ResearchSignal};
pub use fetch::MarketResearchClient;
pub use source::{ResearchOptions, load_sources};
