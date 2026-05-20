# Agent Contract

This repository is an agentic betting workflow for daily Norsk Tipping candidate
selection. It combines deterministic Rust agents, OpenAI review agents, GitHub
Actions automation, and GitHub Pages publishing.

## Mission

Produce a daily top-5 shortlist of bets that:

- use Norsk Tipping as the final bet price,
- prefer the configured odds band, default `1.10-1.30`,
- never research or rank prices below `1.10` or above the hard `1.35` ceiling,
- focus on football/soccer by default,
- rank by probability, confidence, context risk, and research,
- use optional independent signals when available,
- explain strength and risk,
- output top-5 best available candidates when candidates exist,
- output `NO BET` only when there are no candidates to rank.

## Execution Layers

`Deterministic Rust Layer`

- Implemented in `src/agents/`.
- Loads same-day football/soccer candidates from live Norsk Tipping Oddsen by
  default in the scheduled publisher.
- Optionally enriches candidates from `reference_odds.csv` or the env-gated
  The Odds API provider before scoring.
- Optionally enriches football context from the env-gated API-Football provider
  before scoring.
- Uses The Odds API `totals` only when `BETTING_ODDS_API_MARKETS` explicitly
  includes it; scheduled defaults stay on `h2h` to conserve credits.
- Uses The Odds API `double_chance` only when `BETTING_ODDS_API_MARKETS`
  explicitly includes it; event-level odds requests are capped.
- Limits The Odds API bookmaker requests to at most 5 explicit bookmaker keys;
  the default set is Unibet SE, Pinnacle, Betfair Exchange EU, Betsson, and
  William Hill.
- Reports provider request and match counts without exposing `BETTING_ODDS_API_KEY`.
- Reports API-Football request and match counts without exposing
  `BETTING_FOOTBALL_DATA_API_KEY`.
- Filters candidates.
- Allows several preferred markets from the same match when the football board
  has fewer than 5 separate matches.
- Calculates probability, confidence, contextual risk, and optional value/edge.
- Adds a structured football context checklist for form, injuries/suspensions,
  motivation, schedule/travel, and market context.
- Applies capped learning adjustments from settled historical football buckets
  when enough similar win/loss picks exist.
- Does not require external comparison odds for live Norsk Tipping candidates.
- Produces the compact top-5 deterministic report.

`OpenAI Review Layer`

- Implemented in `src/ai/mod.rs`.
- Enabled in GitHub Actions when `OPENAI_API_KEY` is configured.
- Uses `gpt-5.5` by default.
- Runs four role calls in sequence: Explorer, Reviewer, Risk Manager, Output
  Writer.

`Automation Layer`

- Daily report workflow: `.github/workflows/daily-report.yml`.
- Security workflow: `.github/workflows/security-guardrails.yml`.
- Static publisher: `scripts/publish_static_report.sh`.
- Football research sources: `examples/football_research_sources.txt`.
- Optional result settlements: explicit `BETTING_SETTLEMENTS_JSONL` JSON Lines
  records only.
- Published report and history paths:

```text
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/today.txt
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/history.jsonl
```

## Agent Roles

`Explorer`

- Finds the strongest available candidate evidence.
- Reviews probability, context risk, confidence, optional value/edge, and
  research.
- Reviews football context categories for form, injuries/suspensions,
  motivation, schedule/travel, and market context.
- Reviews the learning note without treating no-history or insufficient-history
  output as support.
- Flags missing context that affects confidence.

`Reviewer`

- Challenges the ranking.
- Finds weak evidence and overclaiming.
- Flags stale or missing football context and overstated learning claims.
- Distinguishes likely bets from bets with proven external edge.

`Risk Manager`

- Reviews downside risk, missing data, and no-bet triggers.
- Downgrades or questions unresolved injury/suspension, motivation,
  schedule/travel, market context, and insufficient learning support.
- Checks confidence against evidence.
- Prevents guarantee language.

`Output Writer`

- Writes the final concise report.
- Includes sport, event, kickoff time, market, selection, Norsk Tipping odds,
  comparison, rationale, football checklist, learning note, risk, and
  confidence.

## Hard Rules

- Never invent odds, injuries, probabilities, teams, markets, sources, or
  research findings.
- Never label a fallback candidate as a strict recommendation.
- Never use reference odds as the final bet price.
- Never use stale fixture reference odds as live comparison data.
- Never treat social posts or betting pages as proof.
- Never infer final results from unstructured text; settlement requires an
  explicit checked result source.
- Never imply a guaranteed win.
- Keep secrets out of git.
- Keep public internet automation outside cleared or classified environments
  unless explicitly approved.

## Visible Agent Definitions

- `.agents/workflows/daily_betting.md`
- `.agents/roles/deterministic_rust_agents.md`
- `.agents/roles/explorer.md`
- `.agents/roles/reviewer.md`
- `.agents/roles/risk_manager.md`
- `.agents/roles/output_writer.md`

## Secrets

GitHub Actions:

- `BETTING_REPORT_TOKEN`
- `OPENAI_API_KEY`
- `BETTING_ODDS_API_KEY` when GitHub Actions should enrich h2h/main-market
  football prices from The Odds API.
- `BETTING_FOOTBALL_DATA_API_KEY` when GitHub Actions should enrich football
  form, injury/suspension, and schedule context from API-Football.
- `BETTING_PUSHOVER_TOKEN` and `BETTING_PUSHOVER_USER` when GitHub Actions
  should send iPhone push notifications.

Local `.env` is ignored by git and may contain local-only credentials.

## Validation

Before pushing meaningful changes, run:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
scripts/security_static_checks.sh
cargo audit
```

For static publishing without API billing:

```bash
BETTING_REPORT_TOKEN=test-token \
BETTING_PUBLIC_DIR=/tmp/betting-public \
BETTING_ENABLE_AI=false \
scripts/publish_static_report.sh
```
