# Deterministic Rust Agents

These are implemented in `src/agents/`.

## OddsScreeningAgent

File: `src/agents/filter.rs`

- Filters candidates by date.
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

## DailySelectionAgent

File: `src/agents/selector.rs`

- Applies hard gates.
- Scores candidates.
- Selects the top 3 candidates.
- Fills with best available fallback candidates when fewer than 3 pass every
  strict gate, preferring candidates inside the configured odds band.
- Returns `NO BET` only when there are no candidates to rank.

## Orchestrator

File: `src/agents/mod.rs`

- Runs the deterministic pipeline end to end.
- Feeds the final compact report into the optional AI agent workflow.
