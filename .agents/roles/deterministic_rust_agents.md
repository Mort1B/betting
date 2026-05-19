# Deterministic Rust Agents

These are implemented in `src/agents/`.

## OddsScreeningAgent

File: `src/agents/filter.rs`

- Filters candidates by date.
- Keeps football/soccer candidates by default.
- Enforces the configured Norsk Tipping odds band.

## ProbabilityModelAgent

File: `src/agents/probability.rs`

- Estimates probability from model probability and/or reference odds.
- Uses Norsk Tipping market-implied probability as the default success baseline.
- Does not claim external value edge unless model probability or reference odds
  are supplied.

## Reference Odds Enrichment

File: `src/reference.rs`

- Applies optional external comparison prices before agent scoring.
- Matches reference rows by candidate id or normalized event, market, and
  selection.
- Preserves Norsk Tipping as the final bet price.

## ValueAgent

File: `src/agents/value.rs`

- Calculates implied probability.
- Calculates edge.
- Calculates expected value.

## RiskAgent

File: `src/agents/risk.rs`

- Applies confidence penalties.
- Flags risk terms from sport, competition, event, market, selection, and notes.
- Integrates research warning signals.
- Integrates football context warnings for form, injuries/suspensions,
  lineup/rotation, motivation, schedule/travel, weather/venue, and market
  context.

## LearningAgent

File: `src/agents/learning.rs`

- Reads settled pick history from `BETTING_HISTORY_INPUT`.
- Applies same-run explicit settlements from `BETTING_SETTLEMENTS_JSONL` before
  bucket calculation.
- Ignores pending, void, and unknown results.
- Requires at least 5 similar settled win/loss picks before adjustment.
- Caps confidence movement to +/-3 percentage points and emits a visible note.

## DailySelectionAgent

File: `src/agents/selector.rs`

- Applies hard gates.
- Scores candidates.
- Selects the top 5 candidates by default.
- Fills with best available fallback candidates when fewer than 5 pass every
  strict gate, preferring candidates inside the configured odds band.
- Returns `NO BET` only when there are no candidates to rank.

## Orchestrator

File: `src/agents/mod.rs`

- Runs the deterministic pipeline end to end.
- Feeds the final compact report into the optional AI agent workflow.
