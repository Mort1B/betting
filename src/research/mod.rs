mod analyze;
mod fetch;
mod source;

pub use analyze::{ResearchDigest, assess_candidate_research};
pub use fetch::MarketResearchClient;
pub use source::{ResearchOptions, load_sources};
