# Football Daily Picks Implementation Plan

## Objective

Move the daily workflow from an all-sports top-3 shortlist to a football-only
top-5 shortlist that learns from previous picks and forces the research layer to
factor in football context before a pick is presented.

Target behavior:

- only football/soccer candidates are considered,
- Norsk Tipping remains the final bet price,
- the preferred odds band is `1.10-1.30`, with `1.30-1.35` as fallback-only
  slack,
- the report returns 5 ranked football picks when at least 5 rankable football
  candidates exist,
- the report returns fewer than 5 only when fewer football candidates exist,
- `NO BET` is used only when there are no football candidates to rank,
- previous pick outcomes inform future confidence and risk adjustments,
- research evidence is organized around form, injuries, suspensions, lineups,
  motivation, schedule pressure, market context, and other football-specific
  risks.

## Current Gaps

- `src/norsk_tipping/mod.rs` currently loops through every sport returned by
  Norsk Tipping.
- `src/agents/selector.rs`, `src/report.rs`, `src/ai/mod.rs`, `README.md`,
  `AGENTS.md`, and `.agents/` still describe or enforce top-3 behavior.
- Research matching currently detects generic positive and warning terms, but it
  does not produce a structured football checklist per candidate.
- Previous picks are not stored in a persistent history file, and no learning
  adjustment is applied from past results.
- GitHub Pages deployment currently publishes `today.txt` and the current dated
  report, but it does not preserve a durable pick history for later runs.

## Implementation Slices

### 1. Football-Only Candidate Scope

Status: implemented.

Add a football scope before ranking.

- Add a sport filter to the live Norsk Tipping loader.
- Treat `Fotball`, `Football`, and `Soccer` as accepted football names.
- Apply the same scope to CSV fallback input so tests and manual runs match live
  behavior.
- Add `BETTING_SPORT_SCOPE=football` and a CLI flag such as
  `--sport-scope football`, with football as the scheduled default.
- Update error and no-candidate messages so they say when no football candidates
  were available.

Acceptance criteria:

- Non-football live sport types are skipped before event candidates are ranked.
- CSV rows with non-football sports are excluded when football scope is active.
- Unit tests cover Norwegian and English football names.

### 2. Daily Pick Count

Status: implemented.

Replace hard-coded top-3 behavior with configurable top-5 behavior.

- Add `pick_count` to the rules or selection options.
- Default `pick_count` to `5`.
- Add `BETTING_PICK_COUNT=5` and a CLI flag such as `--pick-count 5`.
- Replace hard-coded `3` limits in `DailySelectionAgent`.
- Update report headers, fallback messages, AI instructions, README, AGENTS, and
  `.agents` role docs from top-3 to top-5.
- Keep fallback labels when fewer than 5 candidates pass strict gates.

Acceptance criteria:

- The selector returns 5 candidates when 5 or more candidates are rankable.
- Fallback copy says `top 5`, not `top 3`.
- AI output writer instructions require 5 candidates when available.

### 2a. Expanded Football Markets And Slack Odds

Status: implemented.

Support daily top-5 output even when fewer than 5 separate football matches are
played.

- Keep football/soccer as the only scheduled sport scope.
- Keep `1.10-1.30` as the preferred odds band.
- Exclude prices below `1.10` or above `1.35` before ranking.
- Allow `1.30-1.35` only as fallback slack.
- Add supported expanded market extraction for goals, corners, cards, both
  teams to score, and player scorer markets.
- Let multiple markets from the same match appear when the match board is thin.
- Apply extra risk penalties to volatile expanded markets.

Acceptance criteria:

- Live Norsk Tipping import can build supported expanded football market
  candidates.
- Candidates outside `1.10-1.35` are skipped before candidate-specific ranking.
- Slack candidates are visible as fallback-only and do not pass strict gates.

### 3. Football Context Research

Status: implemented.

Make football research structured enough that the agents cannot ignore the
important context.

- Add a `FootballContextAgent` or equivalent deterministic module after market
  research and before final selection.
- Expand research analysis into categories:
  - recent form and home/away form,
  - injuries and suspensions,
  - likely lineups and rotation,
  - motivation such as title race, promotion, relegation, European qualification,
    cup priority, or dead-rubber risk,
  - schedule congestion, travel, fatigue, and short rest,
  - weather, pitch, venue, derby/rivalry, and manager-change context,
  - market movement or price disagreement when supplied.
- Store category findings as structured candidate context, not only free-text
  notes.
- Adjust confidence and risk from category findings with small, visible
  penalties or bonuses.
- Add an explicit `unknown` state when the research did not find reliable
  candidate-specific evidence for a category.
- Treat social posts and betting pages as weak signals, not proof.
- Do not invent missing team news, injuries, motivation, or form.

Acceptance criteria:

- Every reported pick includes a football context checklist.
- A candidate with warning evidence for injuries, rotation, or motivation is
  downgraded in a transparent way.
- A candidate with no candidate-specific research does not receive a fabricated
  context boost.

### 4. Research Sources For Football

Status: implemented.

Make the research input match the football-only workflow.

- Add a football-specific source file, for example
  `examples/football_research_sources.txt`.
- Keep source rows in the existing `name|kind|url` format unless a schema change
  is needed.
- Include sources that can provide team news, match previews, table motivation,
  injuries, suspensions, and form.
- Update the scheduled publisher to use the football research source file.
- Keep the current max page default at 10 unless the implementation shows a
  need to increase it.

Acceptance criteria:

- The daily workflow reads football-specific sources by default.
- The source file is explicit enough for a maintainer to add or remove sources
  without changing Rust code.
- Research fetch failures are visible but do not fabricate data.

### 5. Previous Pick History

Status: implemented.

Add durable history so the workflow can learn from earlier recommendations.

- Define a `PickHistoryEntry` schema with:
  - report date,
  - rank,
  - candidate id,
  - sport,
  - competition,
  - event,
  - market,
  - selection,
  - Norsk Tipping odds,
  - score and confidence at pick time,
  - football context category summary,
  - result status: `pending`, `win`, `loss`, `void`, or `unknown`,
  - settlement source, source URL, and timestamp when known.
- Publish a token-protected history file beside `today.txt`, for example
  `/<BETTING_REPORT_TOKEN>/history.jsonl`.
- Before publishing, fetch the previous history file from GitHub Pages when it
  exists, merge the new picks, and republish the combined file.
- Keep the merge idempotent by keying entries on date, event, market, selection,
  and start time.
- Never overwrite a settled result with an unknown result.

Acceptance criteria:

- A new daily run appends the 5 current picks once.
- Re-running the same day does not duplicate those picks.
- Missing historical data starts from an empty history without failing the daily
  report.

### 6. Result Settlement

Status: implemented.

Resolve previous picks only from verifiable result data.

- Add a result updater that checks unsettled football picks after kickoff.
- Prefer a reliable Norsk Tipping result source if available.
- If Norsk Tipping result data is not available, use a configured public result
  source and store its source name and URL.
- Leave results as `pending` or `unknown` when a verified result is unavailable.
- Support void/postponed/cancelled outcomes without counting them as losses.

Acceptance criteria:

- The workflow can update old picks from `pending` to a verified final status.
- The learning layer ignores pending and unknown results.
- No result is inferred from incomplete or untrusted text.

### 7. Learning Agent

Status: implemented.

Use previous picks to adjust future ranking carefully.

- Add a deterministic `LearningAgent` that reads settled history.
- Compute calibration by stable buckets such as:
  - competition,
  - market type,
  - odds range,
  - favorite/underdog/draw selection type,
  - home or away selection when detectable,
  - context warning categories.
- Require a minimum sample size before applying a bucket adjustment.
- Cap learning adjustments so history cannot overpower current odds, form,
  injuries, and motivation.
- Emit a short learning note per pick, for example:
  `history: similar bucket 18 settled picks, 78% hit rate, +2 confidence`.
- Keep history-based changes transparent in the report.

Acceptance criteria:

- With no settled history, scoring is unchanged except for a note that no
  learning data was available.
- With enough settled history, confidence and score adjustments are small,
  deterministic, and visible.
- Tests cover positive, negative, and insufficient-sample history buckets.

### 8. AI Agent Prompt Updates

Status: implemented.

Make the OpenAI review layer inspect the same football context as the
deterministic layer.

- Update Explorer instructions to summarize evidence for form, injuries,
  suspensions, motivation, lineup risk, schedule pressure, and market context.
- Update Reviewer instructions to challenge weak football context and flag
  missing or stale research.
- Update Risk Manager instructions to downgrade candidates when team news,
  motivation, lineup, or schedule risk is unresolved.
- Update Output Writer instructions to produce top 5 and include the football
  checklist and learning note.
- Keep the rule that agents use supplied evidence only.

Acceptance criteria:

- AI output cannot claim form, injury, or motivation facts unless they appear in
  the deterministic report or prior agent output.
- The final output includes 5 picks when the deterministic report has 5 picks.
- The final output preserves fallback and uncertainty warnings.

### 9. Report And Documentation

Status: implemented.

Update user-facing text only after the behavior exists.

- Update `README.md`, `AGENTS.md`, `docs/ARCHITECTURE.md`, `docs/AI_AGENTS.md`,
  and `.agents/` role files from all-sports top-3 to football top-5.
- Add a report section for:
  - football-only scope,
  - pick history status,
  - learning summary,
  - per-pick football context checklist,
  - source coverage and missing context.
- Keep the report concise enough for the iPhone shortcut.

Acceptance criteria:

- Docs match actual behavior after implementation.
- The generated `today.txt` is still readable on a phone.
- The report does not hide weak or fallback candidates.

### 10. Validation

Status: implemented.

Run the normal validation suite before pushing meaningful changes.

Required commands:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
scripts/security_static_checks.sh
cargo audit
```

Static publishing smoke test:

```bash
BETTING_REPORT_TOKEN=test-token \
BETTING_PUBLIC_DIR=/tmp/betting-public \
BETTING_ENABLE_AI=false \
BETTING_SPORT_SCOPE=football \
BETTING_PICK_COUNT=5 \
scripts/publish_static_report.sh
```

## Suggested Order

1. Add football-only candidate filtering.
2. Replace top-3 with configurable top-5 selection.
3. Add structured football context research.
4. Add football-specific research sources.
5. Add pick history persistence.
6. Add result settlement.
7. Add learning adjustments.
8. Update AI prompts.
9. Update docs and visible agent contracts.
10. Run full validation and static publishing smoke test.

## Non-Goals For The First Implementation

- Do not widen the hard `1.35` research ceiling unless explicitly requested.
- Do not use another bookmaker as the final bet price.
- Do not create guaranteed-bet language.
- Do not invent injuries, motivation, team news, outcomes, or sources.
- Do not let learning from a small sample dominate today's actual research.
