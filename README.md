# Betting Daily Agent

Daily agentic betting workflow for finding the strongest Norsk Tipping
candidates by success probability, confidence, context risk, and research. The
scheduled workflow focuses on football/soccer by default, and the final bet
price is always the current Norsk Tipping odds.

The daily target is a top-5 football shortlist in the preferred odds band,
default `1.10-1.30`. Prices from `1.30-1.35` are fallback-only slack, and the
tool does not research or rank prices below `1.10` or above `1.35`.

## Current Setup

- Rust CLI for deterministic candidate scoring.
- Live Norsk Tipping Oddsen loader for same-day candidates.
- CSV candidate input for fixtures, manual runs, and fallback testing.
- Structured football context checks for form, injuries/suspensions, lineups,
  motivation, schedule/travel, weather/venue, and market context.
- Deterministic learning from settled historical picks, capped so history cannot
  overpower current context.
- Visible agent definitions in `.agents/`.
- Four OpenAI API review agents: `Explorer`, `Reviewer`, `Risk Manager`, and
  `Output Writer`.
- GitHub Actions runs the daily report workflow.
- GitHub Pages publishes the report to a tokenized `today.txt` URL.
- GitHub Pages also publishes tokenized `history.jsonl` pick history for future
  learning and audit work.
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
22:00 UTC daily
```

That is 16:00 and midnight in Norway during daylight saving time.

Required GitHub Actions secrets:

- `BETTING_REPORT_TOKEN`: long random token for the private-by-obscurity report
  path.
- `OPENAI_API_KEY`: paid OpenAI API key for the four-agent review.

Report URL shape:

```text
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/today.txt
```

Pick history URL shape:

```text
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/history.jsonl
```

## Local Run

Deterministic report:

```bash
cargo run -- --norsk-tipping-live \
  --date 2026-05-16 \
  --research examples/football_research_sources.txt
```

OpenAI reviewed report:

```bash
OPENAI_API_KEY=... cargo run -- --norsk-tipping-live \
  --date 2026-05-16 \
  --research examples/football_research_sources.txt \
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

Live source controls:

- `BETTING_CANDIDATE_SOURCE=norsk-tipping-live` uses the public Norsk Tipping
  Oddsen sportsbook content endpoint. This is the scheduled default.
- `docs/NORSK_TIPPING_API_PORTAL.md` records the 2026-05-19 API portal
  discovery result. The portal is not integrated because anonymous access did
  not expose usable sportsbook API docs or a stable OpenAPI export.
- `BETTING_SPORT_SCOPE=football` keeps scheduled and local default runs focused
  on football/soccer. Use `all` only for manual all-sports diagnostics.
- `BETTING_PICK_COUNT=5` controls how many ranked picks the report should return
  when enough candidates exist.
- `BETTING_MIN_ODDS=1.10` and `BETTING_MAX_ODDS=1.30` control the preferred
  band. The Rust layer keeps `1.35` as the hard research ceiling.
- `BETTING_NT_EVENTS_PER_SPORT=35` controls how many events are requested per
  sport.
- `BETTING_NT_EARLIEST_START` defaults to `16:00` on the report date.
- `BETTING_NT_LATEST_START` defaults to `05:00` on the next Oslo date. Runs
  before `05:00` keep the report date on the previous day so the midnight
  workflow refreshes the same 16:00-05:00 card.
- `BETTING_REFERENCE_ODDS_CSV=/path/to/reference_odds.csv` optionally adds
  external comparison prices for audit context. It is not required.
- `BETTING_CANDIDATE_SOURCE=csv` uses `BETTING_INPUT_CSV` instead.

## Optional Reference Odds

Live Norsk Tipping prices are the final bet prices. External comparison odds are
optional and are not required for the daily workflow. Use `--reference-odds`
only when you want an extra audit note from your own collected comparison data.

```bash
cargo run -- --norsk-tipping-live \
  --date 2026-05-16 \
  --reference-odds reference_odds.csv \
  --research examples/football_research_sources.txt
```

Reference CSV columns:

- `reference_odds`: required external decimal odds.
- `candidate_id`: optional exact candidate id match.
- `event`, `market`, `selection`: required together when `candidate_id` is not
  supplied.
- `sport`, `competition`: optional extra match constraints.
- `source`, `notes`: optional audit text in the report notes.

When multiple rows match the same candidate, the tool converts each reference
price to implied probability, averages the probabilities, and converts the
consensus back to decimal odds. Existing `reference_odds` in the main candidate
CSV are not overwritten.

## Candidate CSV Fallback

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

- Norsk Tipping preferred odds: `1.10-1.30`; hard research ceiling: `1.35`.
- Minimum estimated probability: `79%`.
- Minimum confidence: `65%`.
- Minimum edge and expected value: enforced only when `model_probability` or
  `reference_odds` is supplied.

Live Norsk Tipping imports use market-implied probability as the success
baseline, then the risk layer adjusts confidence for context such as market type,
sport, entertainment/special markets, friendlies, injuries, rotation, weather,
research warnings, and structured football context warnings. This keeps the
default workflow focused on the best available candidates without requiring
external odds.

Each reported pick includes a football context checklist covering form,
injuries/suspensions, lineup/rotation, motivation, schedule/travel,
weather/venue, and market context. Missing candidate-specific evidence is shown
as `unknown` and does not create a confidence boost.

When previous settled history is available, the learning layer compares today's
pick to stable football buckets such as competition, market type, odds range,
selection type, and warning categories. It requires at least 5 similar settled
win/loss picks before adjusting confidence, caps that adjustment to +/-3
percentage points, and prints the learning note in the report. Pending, void,
and unknown results do not affect learning.

The daily report still returns the top 5 ranked candidates when strict gates or
the date filter would otherwise leave the report empty. If fewer than 5 separate
football matches are available, expanded markets such as goals, corners, cards,
and player scorers can fill the shortlist from the games being played.
Fallback candidates are labelled with their failed strict-rule checks and
include a `Confidence score` out of 100, so the report remains useful without
hiding weak or stale input data.

The report starts with a compact run summary covering football scope, pick
target, pick-history status, source coverage, missing football context, and the
learning summary. Each pick then keeps its own research counts, learning note,
strict status, and football context checklist.

## Agent Workflow

Deterministic Rust agents:

- `OddsScreeningAgent`
- `ProbabilityModelAgent`
- `ValueAgent`
- `RiskAgent`
- `DailySelectionAgent`

OpenAI review agents:

- `Explorer`: finds value evidence and missing comparison data.
- `Reviewer`: challenges ranking, weak football evidence, stale research, and
  overclaiming.
- `Risk Manager`: checks downside risk, unresolved team context, learning
  support, and no-bet triggers.
- `Output Writer`: writes the final concise top-5 report.

Human-readable agent contracts:

- `AGENTS.md`
- `.agents/workflows/daily_betting.md`
- `.agents/roles/*.md`

## Research Sources

Scheduled football research source file:

```text
examples/football_research_sources.txt
```

`examples/research_sources.txt` is kept as a mixed-sport manual diagnostics
file, but scheduled football runs use the football-specific list.

Supported source kinds:

- `reddit_json`
- `html`

Research is treated as weak supporting evidence. It can adjust confidence, but
it must not override hard probability, confidence, and odds-band gates.
Fetch failures are shown as source-error notes so missing research is visible.

## Pick History

The static publisher writes `history.jsonl` beside `today.txt`. Before each
publish it fetches the previous Pages copy, merges the current picks by report
date, event, market, selection, and start time, and republishes the combined
file.

New picks start as `pending`. Manual or future settlement data marked as `win`,
`loss`, or `void` is preserved on same-day reruns so a rerun cannot erase a
settled result. Set `BETTING_HISTORY_URL` only when testing against a custom
previous-history location.

For local learning tests, point `BETTING_HISTORY_INPUT` at a history JSONL file.
If `BETTING_SETTLEMENTS_JSONL` is also set, those checked settlements are applied
in memory before learning buckets are calculated.

## Result Settlement

Set `BETTING_SETTLEMENTS_JSONL=/path/to/settlements.jsonl` to update existing
history rows from explicit settlement records after the report is generated.
The file must use JSON Lines with exact history keys:

```json
{"report_date":"2026-05-15","candidate_id":"ex-001","event":"Rosenborg - Brann","market":"Double chance","selection":"Rosenborg or draw","starts_at":"2026-05-15T18:00:00+02:00","result_status":"win","settlement_source":"checked final result source","settlement_source_url":"https://source-url.example/match","settled_at":"2026-05-16T10:00:00Z"}
```

Accepted statuses are `win`, `loss`, `void`, and `unknown`; `pending`
settlement records are rejected. `settlement_source_url` is optional for manual
checks and should be included when using a public result page. Settled `win`,
`loss`, and `void` history rows are never overwritten by later unknown checks.

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
- `docs/FOOTBALL_DAILY_PICKS_PLAN.md`
- `docs/NORSK_TIPPING_API_PORTAL.md`
- `docs/AI_AGENTS.md`
- `docs/LANGGRAPH_EVALUATION.md`
- `docs/MORNING_DELIVERY.md`

## Responsible Use

This is decision support, not a guarantee. Do not chase losses, increase stake
size after losses, or force a bet when the slate is poor.
