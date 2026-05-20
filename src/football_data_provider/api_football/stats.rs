#[derive(Debug, Default)]
pub(super) struct ProviderStats {
    pub(super) fixture_requests: usize,
    pub(super) fixture_success: usize,
    pub(super) injury_requests: usize,
    pub(super) injury_success: usize,
    pub(super) coverage_requests: usize,
    pub(super) coverage_success: usize,
    pub(super) standings_requests: usize,
    pub(super) standings_success: usize,
    pub(super) form_requests: usize,
    pub(super) form_success: usize,
}

impl ProviderStats {
    pub(super) fn summary(&self, matched_candidates: usize, candidate_count: usize) -> String {
        format!(
            "API-Football: fixture requests {}/{}, coverage requests {}/{}, injury requests {}/{}, standings requests {}/{}, form team requests {}/{}, matched {matched_candidates}/{candidate_count} candidate(s)",
            self.fixture_success,
            self.fixture_requests,
            self.coverage_success,
            self.coverage_requests,
            self.injury_success,
            self.injury_requests,
            self.standings_success,
            self.standings_requests,
            self.form_success,
            self.form_requests
        )
    }
}
