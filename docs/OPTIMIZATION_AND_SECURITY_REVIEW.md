# Optimization Plan And Security Review

Date: 2026-05-19

Scope: repository-wide review of the daily Norsk Tipping betting workflow. This
document is a planning and review artifact only. It does not change runtime
behavior.

## Current Baseline

- The scheduled workflow runs on `main`, loads live Norsk Tipping football
  candidates, fetches configured research sources, optionally runs OpenAI review
  agents, publishes GitHub Pages output, and can send Pushover delivery.
- Validation passed during this review:
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy --all-targets -- -D warnings`
  - `scripts/security_static_checks.sh`
  - `cargo audit`
- The repository is public by design. Published report and history files are
  protected only by a long random URL path token.

## Optimization Principles

- Measure before changing behavior.
- Keep deterministic output order stable, especially for ranked candidates and
  research source notes.
- Preserve the hard betting rules: Norsk Tipping is the final price, hard odds
  ceiling remains `1.35`, and `NO BET` is only for an empty candidate set.
- Keep every source file under the repository's 400-line policy.
- Validate every optimization with tests plus at least one static-publisher
  smoke run with AI disabled.

## Optimization Plan

### 1. Add Stage Timing

Status: completed on 2026-05-19. CLI parsing moved to `src/cli.rs` to keep
`src/main.rs` below the 400-line policy, and timing output is gated by
`BETTING_TIMINGS`.

Files:
- `src/main.rs`
- `scripts/publish_static_report.sh`

Plan:
- Add lightweight timing around candidate loading, reference enrichment,
  research fetch, deterministic scoring, history write, AI review, report
  rendering, and delivery.
- Print timings to stderr so published `today.txt` stays stable.
- Gate detailed timing behind an environment variable such as
  `BETTING_TIMINGS=true`.
Impact: high diagnostic value with low behavior risk.
Validation:
- `cargo test`
- static publisher with `BETTING_ENABLE_AI=false`
- one manual AI-enabled run when cost is acceptable

### 2. Parallelize Bounded Research Fetching

Status: completed on 2026-05-19. Research sources are fetched in deterministic
batches of four.

Files:
- `src/research/fetch.rs`
- `src/research/source.rs`
- `examples/football_research_sources.txt`

Plan:
- Fetch configured research sources concurrently with a small fixed worker
  limit, for example 4.
- Preserve deterministic output order by carrying source indexes and sorting
  results before constructing `ResearchDigest`.
- Keep per-source timeout behavior and visible source-error notes.

Impact: high. The current implementation fetches up to 10 sources sequentially,
with a 15 second timeout per source.
Risk: medium. Ordering and error reporting must remain stable.
Validation:
- Add tests for stable source ordering after mixed success and failure.
- Add a local mock server test for slow and failed sources.
- Compare wall-clock time before and after on a normal daily source set.

### 3. Normalize Research Text Once

Status: completed on 2026-05-19. `ResearchPage` now stores cached normalized
search text reused by research and football context matching.

Files:
- `src/research/analyze.rs`
- `src/football_context.rs`

Plan:
- Store normalized searchable text on `ResearchPage`, or add an indexed
  research view built once after fetching.
- Avoid rebuilding `format!("{} {}", title, text).to_lowercase()` per candidate.
- Reuse candidate terms between generic research and football context where the
  required term sets are compatible.
Impact: high as candidate count grows.
Risk: medium. Matching semantics must remain equivalent.

Validation:
- Add fixture-based parity tests for research matches, warning mentions,
  price hints, and football context categories.
- Snapshot a deterministic fixture report before and after.

### 4. Precompute Learning Buckets Once

Status: completed on 2026-05-19. `LearningAgent` now builds settled bucket
counts at construction time.

Files:
- `src/agents/learning.rs`

Plan:
- Build settled-entry references and bucket counts when constructing
  `LearningAgent`.
- Keep `assess()` focused on computing the candidate bucket list and looking up
  precomputed counts.
Impact: high once `history.jsonl` grows.
Risk: low to medium.

Validation:
- Existing learning tests.
- Add a multi-candidate test proving adjustments are identical before and after
  precomputation.

### 5. Consolidate History And Settlement State
Status: completed on 2026-05-19. Run-level `HistoryState` now loads history and settlements once, feeds learning, and writes merged output in memory.
Files:
- `src/history_pipeline.rs`
- `src/history.rs`
- `src/agents/learning.rs`
- `src/settlement.rs`

Plan:
- Introduce a single run-level history state that loads prior history, applies
  explicit settlements, feeds learning, and writes merged output.
- Avoid reading and applying settlement data separately for learning and output.

Impact: medium. This reduces duplicate I/O and makes history behavior easier to
audit.
Risk: medium because settled result preservation is important.
Validation:
- History merge tests.
- Settlement preservation tests.
- Static publisher smoke with `BETTING_SETTLEMENTS_JSONL`.

### 6. Reduce AI Token And Round-Trip Cost
Status: completed on 2026-05-19. AI review now uses compact role inputs and a mockable client boundary while preserving four calls.
Files:
- `src/ai/mod.rs`
- `.github/workflows/daily-report.yml`

Plan:
- Add a compact structured deterministic summary for AI input.
- Keep the four visible roles, but pass only the fields each role needs.
- Consider a mode that combines Explorer and Reviewer into one API request only
  after measuring quality and cost impact.
- Keep `store: false` and capped `max_output_tokens`.
Impact: high when AI is enabled.
Risk: medium to high. Role separation is part of the product behavior.

Validation:
- Add a mock OpenAI client or transport boundary.
- Assert that required report sections survive the compact-input path.
- Compare token estimate and output quality on the same deterministic fixture.

### 7. Cache CI Build And Audit Work

Files:
- `.github/workflows/daily-report.yml`
- `.github/workflows/security-guardrails.yml`

Plan:
- Add Rust/Cargo cache usage for both workflows.
- Pin or cache `cargo-audit` instead of installing an unversioned latest copy on
  every security run.
Impact: medium to high CI runtime reduction.
Risk: low.

Validation:
- Compare GitHub Actions duration before and after.
- Confirm clean cache-miss runs still pass.

### 8. Index Reference Odds Matching

Files:
- `src/reference.rs`

Plan:
- Build lookup maps for `candidate_id` and normalized
  `event|market|selection` matches.
- Preserve optional sport and competition constraints.
- Keep consensus odds behavior unchanged.
Impact: medium for larger reference files, low for the default scheduled run.
Risk: medium because matching must remain exact enough for audit use.

Validation:
- Expand tests for id match, tuple match, optional sport and competition,
  duplicate consensus, and no-match behavior.

### 9. Split Near-Limit Modules

Files:
- `src/main.rs`
- `src/report.rs`
- `src/domain.rs`
- `src/agents/learning.rs`

Plan:
- Split CLI parsing out of `src/main.rs`.
- Split report rendering helpers out of `src/report.rs`.
- Split domain types by responsibility before adding more scoring or report
  features.
Impact: medium maintainability improvement.
Risk: medium if done as a broad refactor. Prefer mechanical move-only slices.

Validation:
- `cargo fmt --check`
- `cargo test`
- report snapshot parity

## Recommended Iteration Order

1. Instrument stage timings.
2. Parallelize research fetching with deterministic output order.
3. Precompute learning buckets.
4. Normalize research text once.
5. Consolidate history state.
6. Add CI cache and pin audit tooling.
7. Reduce AI input size.
8. Index reference odds.
9. Split near-limit modules as separate maintenance slices.

## Security Threat Model

Assets:
- GitHub Actions secrets: `BETTING_REPORT_TOKEN`, `OPENAI_API_KEY`, and optional
  Pushover credentials.
- Published report and `history.jsonl`.
- Integrity of ranked betting output.
- GitHub Pages deployment permissions.

Trust boundaries:
- Public internet data from Norsk Tipping, Reddit, and betting research pages.
- GitHub Actions runner environment and repository contents.
- OpenAI API request and response boundary.
- Local `.env` used by manual scripts.
- Public GitHub Pages output protected only by URL entropy.

Primary failure modes:
- Secret leakage into git, logs, or Pages output.
- Prompt or research injection causing invented or overstated report claims.
- External research source causing excessive runtime, memory use, or daily
  workflow failure.
- Supply-chain drift in GitHub Actions tooling.
- Accidental publication of sensitive personal data in report history.

## Security Review Findings

### High: Confirm Local OpenAI API Key Rotation

Files:
- `.env`
- `.gitignore`

Issue:
- The local ignored `.env` contained a non-empty OpenAI API key during review.
- The key was removed from local `.env` before this document was pushed.
- `.env` is correctly ignored and tracked-file scans did not find committed
  secret material, but any previously exposed key should still be treated as
  exposed until it has been revoked or rotated.

Recommendation:
- Confirm that key has been revoked or rotated in the OpenAI dashboard.
- Keep local `.env` owner-readable only on machines that run cron.

### Medium: Split Generate And Deploy Privileges

Files:
- `.github/workflows/daily-report.yml`
- `scripts/publish_static_report.sh`

Issue:
- The report-generation job runs repository code with runtime secrets and also
  has `pages: write` plus `id-token: write`.
- A compromised dependency build script or future malicious main-branch code
  would execute in the deploy-capable job.

Recommendation:
- Split generation and deployment into separate jobs.
- Give Pages and OIDC permissions only to the deploy job.
- Use `actions/checkout` with `persist-credentials: false` where possible.

### Medium: External Inputs Need Byte Caps

Files:
- `src/research/fetch.rs`
- `src/norsk_tipping/client.rs`
- `scripts/publish_static_report.sh`
- `src/history.rs`

Issue:
- Research pages, Reddit JSON, Norsk Tipping JSON, previous `history.jsonl`, and
  local history reads have timeouts or structured parsing but no explicit size
  ceilings.
- A hostile or oversized upstream body can exhaust memory, disk, or CPU and
  block daily publishing.

Recommendation:
- Add response byte caps, content-type checks where useful, and history file
  size caps.
- Surface oversize data as source-error or history-error notes instead of
  inventing evidence.

### Low: Research Source URLs Are Not Restricted

Files:
- `src/research/source.rs`
- `src/research/fetch.rs`

Issue:
- Research source rows accept arbitrary URL strings. Current scheduled sources
  are repo-controlled, so SSRF is not reportable as-is, but this should be
  hardened before more dynamic source configuration is added.

Recommendation:
- Accept only `https://` URLs by default.
- Consider an allowlist for known research hostnames.

### Low: Public Pages Output Is Tokenized, Not Authenticated

Files:
- `scripts/publish_static_report.sh`
- `docs/GITHUB_PAGES_SHORTCUT.md`

Issue:
- `today.txt` and `history.jsonl` are public to anyone with the tokenized URL.
- This is intentional and documented as private-by-obscurity.

Recommendation:
- Keep stake sizes, account data, personal data, and secrets out of reports.
- Add token rotation instructions.
- Use Pushover-only delivery if report content becomes sensitive.

### Low: Local `.env` Is Shell-Sourced

Files:
- `scripts/daily_betting.sh`

Issue:
- Local cron execution uses `source "$ENV_FILE"`, which executes shell code.
- This is a local operator risk, not a remote repository vulnerability.

Recommendation:
- Keep `.env` owner-only readable and writable on cron hosts.
- Replace shell sourcing with a strict key-value parser if the host is shared.

### Low: CI Tooling Uses Floating Installer State

Files:
- `.github/workflows/security-guardrails.yml`
- `.github/workflows/daily-report.yml`

Issue:
- Workflows use major-version action tags and install `cargo-audit` at runtime.

Recommendation:
- Pin GitHub Actions to immutable SHAs if stricter supply-chain control is
  desired.
- Pin or cache a known-good `cargo-audit` version.

## Security Non-Issues And Existing Controls

- Secret scanning guardrail is present in `scripts/security_static_checks.sh`.
- `.env` and `public/` are checked by static guardrails as files that should not
  be tracked.
- OpenAI requests use bearer auth and set `"store": false`.
- The AI layer receives deterministic report text, not raw HTML pages.
- Prompt text instructs agents not to invent odds, injuries, results, or
  sources.
- Settlement requires explicit JSON Lines records and rejects `pending` results.
- Research fetch errors are surfaced as source-error notes.
- Shell command construction uses arrays and quoting; no obvious shell injection
  path was found.

## Security Follow-Up Plan

1. Confirm the removed local OpenAI key has been revoked or rotated.
2. Split generation and deployment privileges in the daily workflow.
3. Add response and history byte caps.
4. Validate research source URL schemes and optionally hostnames.
5. Add report-token rotation guidance and keep public-output sensitivity limits
   explicit.
6. Update security docs to list optional Pushover secrets.
7. Pin or cache CI security tooling.
8. Keep the full validation command set required before pushing meaningful
   changes.
