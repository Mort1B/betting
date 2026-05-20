mod the_odds_api;

use crate::domain::BetCandidate;
use crate::reference::ReferenceOddsRow;

pub use the_odds_api::{MAX_BOOKMAKERS, TheOddsApiOptions};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReferenceProviderOptions {
    pub the_odds_api: Option<TheOddsApiOptions>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReferenceProviderOutput {
    pub rows: Vec<ReferenceOddsRow>,
    pub summaries: Vec<String>,
    pub notes: Vec<String>,
}

pub trait ReferenceOddsProvider {
    fn name(&self) -> &'static str;
    fn fetch_rows(&self, candidates: &[BetCandidate]) -> ReferenceProviderOutput;
}

pub fn fetch_reference_provider_rows(
    candidates: &[BetCandidate],
    options: &ReferenceProviderOptions,
) -> ReferenceProviderOutput {
    let mut output = ReferenceProviderOutput::default();

    if let Some(options) = &options.the_odds_api {
        merge_output(
            &mut output,
            the_odds_api::TheOddsApiProvider::new(options.clone()).fetch_rows(candidates),
        );
    }

    output
}

fn merge_output(target: &mut ReferenceProviderOutput, mut source: ReferenceProviderOutput) {
    target.rows.append(&mut source.rows);
    target.summaries.append(&mut source.summaries);
    target.notes.append(&mut source.notes);
}
