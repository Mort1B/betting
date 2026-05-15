# Deterministic Rust Agents

These are implemented in `src/agents/`.

## OddsScreeningAgent

File: `src/agents/filter.rs`

- Filters candidates by date.
- Enforces the configured Norsk Tipping odds band.

## ProbabilityModelAgent

File: `src/agents/probability.rs`

- Estimates probability from model probability and/or reference odds.
- Refuses to treat Norsk Tipping implied probability alone as value evidence.

## ValueAgent

File: `src/agents/value.rs`

- Calculates implied probability.
- Calculates edge.
- Calculates expected value.

## RiskAgent

File: `src/agents/risk.rs`

- Applies confidence penalties.
- Flags risk terms from candidate notes.
- Integrates research warning signals.

## DailySelectionAgent

File: `src/agents/selector.rs`

- Applies hard gates.
- Scores candidates.
- Selects the top 3 bettable candidates or returns `NO BET`.

## Orchestrator

File: `src/agents/mod.rs`

- Runs the deterministic pipeline end to end.
- Feeds the final compact report into the optional AI agent workflow.
