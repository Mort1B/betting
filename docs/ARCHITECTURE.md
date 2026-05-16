# Multi-Agent Betting Architecture

The runtime is a deterministic multi-agent pipeline coordinated by
`DailyBetOrchestrator`.

## Candidate Sources

`Norsk Tipping Live Source`

- Used by the scheduled GitHub Pages publisher by default.
- Requests same-day sport and event boards from the public Oddsen sportsbook
  content endpoint.
- Converts Norsk Tipping fractional price fields into decimal odds.
- Emits candidates only inside the configured odds band, default `1.15-1.30`.
- Skips events earlier than the live-source cutoff passed by the publisher.
- Leaves `model_probability` and `reference_odds` empty by default. The
  probability model can still rank these candidates from market-implied
  probability plus context and research risk.

`CSV Source`

- Kept for fixtures, manual research, fallback tests, and custom candidate
  files.
- Supports `model_probability`, `reference_odds`, `confidence`, and notes.

`Reference Odds Enrichment`

- Runs after candidate loading and before deterministic agents.
- Reads an optional `--reference-odds` CSV.
- Matches by exact `candidate_id`, or by normalized `event`, `market`, and
  `selection` with optional `sport` and `competition` constraints.
- Converts multiple matched external prices into a consensus market-implied
  probability before setting `reference_odds`.
- Does not overwrite `reference_odds` already present in the main candidate
  CSV.

## Agents

`OddsScreeningAgent`

- Applies the daily date filter.
- Enforces the requested Norsk Tipping odds band, default `1.15-1.30`, across
  any sport or league available at Norsk Tipping.
- Keeps rejected candidates visible in the final report so the decision is
  auditable.

`ProbabilityModelAgent`

- Estimates win probability from the best available inputs.
- Uses `model_probability` directly when supplied.
- Uses `reference_odds` as a market-implied probability when supplied.
- Blends model and reference signals when both exist.
- Uses Norsk Tipping market-implied probability as the baseline when no external
  model or reference odds are supplied.

`ValueAgent`

- Calculates implied probability from the Norsk Tipping odds.
- Calculates expected value as `estimated_probability * odds - 1`.
- Calculates edge as `estimated_probability - implied_probability`.
- Reports whether the Norsk Tipping price is higher or lower than supplied
  reference-market odds.

`RiskAgent`

- Starts from the supplied `confidence` score.
- Penalizes contextual risk terms across sport, competition, event, market,
  selection, and notes.
- Penalizes entertainment/special markets, friendlies, injury/rotation/weather
  risk, and research warnings.
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

- Applies hard gates for odds band, probability, and confidence.
- Applies edge and expected-value gates only when independent model/reference
  data exists.
- Scores candidates with a success-first probability and context score.
- Selects the top 3 candidates for the daily report.
- Fills with best available fallback candidates when fewer than 3 pass every
  strict gate, preferring candidates inside the requested Norsk Tipping odds
  band before any outside-band fallback.
- Returns `NO BET` only when there are no candidates to rank.

`OpenAI Agent Workflow`

- Runs only after deterministic filtering and ranking.
- Is enabled in the scheduled GitHub Action when `OPENAI_API_KEY` is configured
  as a repository secret.
- Uses four roles: Explorer, Reviewer, Risk Manager, and Output Writer.
- Passes compact outputs between agents to reduce cost and keep each role
  focused.
- Produces the final user-facing report through the optional `--ai` path.

## Probability And Context

The default workflow does not require external comparison odds. Norsk Tipping's
price gives a market-implied success baseline, then the risk layer adjusts the
candidate for practical context: market type, sport, competition, event notes,
injury/rotation/weather terms, entertainment markets, friendlies, and research
warnings.

If `model_probability` or `reference_odds` is supplied, the system also evaluates
edge and expected value. Without those inputs, the report does not claim a true
external price edge; it ranks the strongest available candidates by probability
and context.

## Daily Workflow

1. Collect current Norsk Tipping candidates in the `1.15-1.30` band from live
   Oddsen data across any available sport or league.
2. Score candidates from market-implied probability, context confidence,
   research signals, and optional model/reference data.
3. Add risk notes after checking injury, lineup, motivation, market type, and
   market context.
4. Run with research enabled:

```bash
cargo run -- --norsk-tipping-live --date YYYY-MM-DD --research examples/research_sources.txt
```

5. Configure `OPENAI_API_KEY` in GitHub Secrets so the scheduled workflow can run
   the four-agent API review. See `docs/OPENAI_API_SETUP.md`.
6. Treat fallback candidates as weaker options and place no bet if the report
   says `NO BET`.
7. For morning delivery, schedule `scripts/daily_betting.sh` with cron and set
   either the SMTP environment variables or the Pushover environment variables.

## Next Extension Points

- Add sport-specific probability agents for football, tennis, hockey, and
  basketball.
- Add a closing-line-value tracker so the daily process can measure whether the
  selected prices beat the later market.
- Add a small local results database to audit hit rate, expected value, and
  calibration over time.
