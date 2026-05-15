# Agent Contract

This repository is an agentic betting workflow for daily Norsk Tipping value
selection. It combines deterministic Rust agents, OpenAI review agents, GitHub
Actions automation, and GitHub Pages publishing.

## Mission

Produce a daily top-3 shortlist of bets that:

- use Norsk Tipping as the final bet price,
- stay inside the configured odds band, default `1.15-1.30`,
- can come from any sport or league,
- compare Norsk Tipping odds against independent signals,
- explain value and risk,
- output `NO BET` when the slate is not good enough.

## Execution Layers

`Deterministic Rust Layer`

- Implemented in `src/agents/`.
- Filters candidates.
- Calculates probability, edge, expected value, and confidence.
- Rejects candidates without independent probability evidence.
- Produces the compact top-3 deterministic report.

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
- Published report path:

```text
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/today.txt
```

## Agent Roles

`Explorer`

- Finds strongest value evidence.
- Compares Norsk Tipping odds against reference odds, model probability, EV,
  edge, and research.
- Flags missing comparison data.

`Reviewer`

- Challenges the ranking.
- Finds weak value evidence and overclaiming.
- Distinguishes likely bets from value bets.

`Risk Manager`

- Reviews downside risk, missing data, and no-bet triggers.
- Checks confidence against evidence.
- Prevents guarantee language.

`Output Writer`

- Writes the final concise report.
- Includes sport, event, market, selection, Norsk Tipping odds, comparison,
  rationale, risk, and confidence.

## Hard Rules

- Never invent odds, injuries, probabilities, teams, markets, sources, or
  research findings.
- Never recommend a candidate that failed deterministic gates.
- Never use reference odds as the final bet price.
- Never treat social posts or betting pages as proof.
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

## Required Secrets

GitHub Actions:

- `BETTING_REPORT_TOKEN`
- `OPENAI_API_KEY`

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
