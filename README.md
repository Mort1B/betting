# Betting Daily Agent

Daily agentic betting workflow for finding the best value candidates on Norsk
Tipping. The system can consider any sport or league, but the final bet price is
always the current Norsk Tipping odds.

The daily target is a top-3 shortlist inside the configured odds band, default
`1.15-1.30`. If the slate is too weak, the correct output is `NO BET`.

## Current Setup

- Rust CLI for deterministic candidate scoring.
- Visible agent definitions in `.agents/`.
- Four OpenAI API review agents: `Explorer`, `Reviewer`, `Risk Manager`, and
  `Output Writer`.
- GitHub Actions runs the daily report workflow.
- GitHub Pages publishes the report to a tokenized `today.txt` URL.
- iPhone Shortcuts can fetch the report URL every day.
- Security guardrails run on every push to `main`.

## Daily Automation

Workflow:

```text
.github/workflows/daily-report.yml
```

Schedule:

```text
14:00 UTC daily
```

That is 16:00 in Norway during daylight saving time.

Required GitHub Actions secrets:

- `BETTING_REPORT_TOKEN`: long random token for the private-by-obscurity report
  path.
- `OPENAI_API_KEY`: paid OpenAI API key for the four-agent review.

Report URL shape:

```text
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/today.txt
```

## Local Run

Deterministic report:

```bash
cargo run -- examples/norsk_tipping_candidates.csv \
  --date 2026-05-16 \
  --research examples/research_sources.txt
```

OpenAI reviewed report:

```bash
OPENAI_API_KEY=... cargo run -- examples/norsk_tipping_candidates.csv \
  --date 2026-05-16 \
  --research examples/research_sources.txt \
  --ai \
  --openai-model gpt-5.5
```

Static publish test:

```bash
BETTING_REPORT_TOKEN=test-token \
BETTING_PUBLIC_DIR=/tmp/betting-public \
BETTING_ENABLE_AI=false \
scripts/publish_static_report.sh
```

## Candidate CSV

Required columns:

- `id`
- `sport`
- `competition`
- `event`
- `market`
- `selection`
- `norsk_tipping_odds`
- `starts_at`

Important optional columns:

- `model_probability`: estimated win probability from your own model or manual
  research.
- `reference_odds`: comparable market price from another source.
- `confidence`: 0.0-1.0 confidence after checking lineup, injury, motivation,
  market stability, and context.
- `notes`: free-text risk/context notes.

## Scoring Rules

Defaults:

- Norsk Tipping odds: `1.15-1.30`.
- Minimum estimated probability: `79%`.
- Minimum edge versus Norsk Tipping implied probability: `1.5` percentage
  points.
- Minimum confidence: `65%`.
- Minimum expected value: `0%`.

A candidate without `model_probability` or `reference_odds` is rejected because
Norsk Tipping implied probability alone cannot prove value.

## Agent Workflow

Deterministic Rust agents:

- `OddsScreeningAgent`
- `ProbabilityModelAgent`
- `ValueAgent`
- `RiskAgent`
- `DailySelectionAgent`

OpenAI review agents:

- `Explorer`: finds value evidence and missing comparison data.
- `Reviewer`: challenges ranking, weak evidence, and overclaiming.
- `Risk Manager`: checks downside risk and no-bet triggers.
- `Output Writer`: writes the final concise top-3 report.

Human-readable agent contracts:

- `AGENTS.md`
- `.agents/workflows/daily_betting.md`
- `.agents/roles/*.md`

## Research Sources

Research source file:

```text
examples/research_sources.txt
```

Supported source kinds:

- `reddit_json`
- `html`

Research is treated as weak supporting evidence. It must not override hard value
and risk gates.

## Security Guardrails

This is a public internet workflow. It talks to GitHub, OpenAI, Reddit, Norsk
Tipping, and public web pages. It must stay outside any cleared enclave unless
all external dependencies and data flows are explicitly approved.

Security docs:

- `SECURITY.md`
- `docs/SECURITY_ARCHITECTURE.md`
- `docs/NETWORK_POLICY.md`
- `docs/CONTAINER_HARDENING_BASELINE.md`
- `docs/GITHUB_SECURITY_GUARDRAILS.md`

Security workflow:

```text
.github/workflows/security-guardrails.yml
```

It runs formatting, tests, clippy, static guardrails, and `cargo audit`.

## Useful Docs

- `docs/OPENAI_API_SETUP.md`
- `docs/GITHUB_PAGES_SHORTCUT.md`
- `docs/AI_AGENTS.md`
- `docs/LANGGRAPH_EVALUATION.md`
- `docs/MORNING_DELIVERY.md`

## Responsible Use

This is decision support, not a guarantee. Do not chase losses, increase stake
size after losses, or force a bet when the slate is poor.
