# Multi-Agent Betting Architecture

The runtime is a deterministic multi-agent pipeline coordinated by
`DailyBetOrchestrator`.

## Agents

`OddsScreeningAgent`

- Applies the daily date filter.
- Enforces the requested Norsk Tipping odds band, default `1.15-1.30`, across
  any sport or league available at Norsk Tipping.
- Keeps rejected candidates visible in the final report so the decision is
  auditable.

`ProbabilityModelAgent`

- Estimates win probability from independent inputs.
- Uses `model_probability` directly when supplied.
- Uses `reference_odds` as a market-implied probability when supplied.
- Blends model and reference signals when both exist.
- Falls back to Norsk Tipping implied probability only for reporting; the
  selector rejects that candidate because it has no independent value signal.

`ValueAgent`

- Calculates implied probability from the Norsk Tipping odds.
- Calculates expected value as `estimated_probability * odds - 1`.
- Calculates edge as `estimated_probability - implied_probability`.
- Reports whether the Norsk Tipping price is higher or lower than supplied
  reference-market odds.

`RiskAgent`

- Starts from the supplied `confidence` score.
- Penalizes missing independent signals and risk terms in notes.
- Applies small confidence adjustments from matched research warnings or
  positive signals.
- Produces explicit risk flags for the report.

`MarketResearchClient`

- Fetches up to 10 configured research sources per run by default.
- Supports Reddit JSON listing URLs and normal HTML pages.
- Extracts the top configured Reddit posts/items from listing sources.
- Produces page-level positive, warning, and decimal-price signals.

`ResearchAssessment`

- Matches research page text against each candidate's event, market, selection,
  and competition terms.
- Adds evidence notes to the final report.
- Treats social and betting-page findings as weak signals, not formal proof of
  value.

`DailySelectionAgent`

- Applies hard gates for probability, value, confidence, and expected value.
- Scores bettable candidates with a success-first value score.
- Selects the top 3 bettable candidates for the daily report.
- Returns `NO BET` when the board does not meet the rules.

`OpenAI Agent Workflow`

- Runs only after deterministic filtering and ranking.
- Uses four `gpt-5.5` roles: Explorer, Reviewer, Risk Manager, and Output
  Writer.
- Passes compact outputs between agents to reduce cost and keep each role
  focused.
- Produces the final user-facing report when `--ai` is enabled.

## Why The Architecture Requires Independent Probability

Norsk Tipping odds already include Norsk Tipping's price and margin. A low odds
selection can be likely to win and still be a poor value bet. The system
therefore refuses to call a candidate valuable unless the CSV includes either:

- `model_probability`, or
- `reference_odds`.

This keeps the daily pick aligned with the goal: the most likely successful bet
inside the odds band that still has value. The sport is not constrained; the
constraint is that the final price must be the current Norsk Tipping price.

## Daily Workflow

1. Collect current Norsk Tipping candidates in the `1.15-1.30` band from any
   sport or league.
2. Add independent model probabilities or reference prices from comparable
   markets.
3. Add confidence and risk notes after checking injury, lineup, motivation, and
   market context.
4. Run with research enabled:

```bash
cargo run -- candidates.csv --date YYYY-MM-DD --research examples/research_sources.txt
```

5. Run with AI enabled when `OPENAI_API_KEY` is available:

```bash
cargo run -- candidates.csv --date YYYY-MM-DD --research examples/research_sources.txt --ai
```

6. Place no bet if the tool says `NO BET`.
7. For morning delivery, schedule `scripts/daily_betting.sh` with cron and set
   either the SMTP environment variables or the Pushover environment variables.

## Next Extension Points

- Add a dedicated Norsk Tipping provider if a stable, permitted export or API is
  available.
- Add sport-specific probability agents for football, tennis, hockey, and
  basketball.
- Add a closing-line-value tracker so the daily process can measure whether the
  selected prices beat the later market.
- Add a small local results database to audit hit rate, expected value, and
  calibration over time.
