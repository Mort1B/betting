# Multi-Agent Betting Architecture

The runtime is a deterministic multi-agent pipeline coordinated by
`DailyBetOrchestrator`.

## Candidate Sources

`Norsk Tipping Live Source`

- Used by the scheduled GitHub Pages publisher by default.
- Requests same-day sport and event boards from the public Oddsen sportsbook
  content endpoint.
- The Norsk Tipping API portal was checked on 2026-05-19 and is not integrated
  because anonymous access does not expose usable sportsbook API documentation
  or a stable OpenAPI export. See `docs/NORSK_TIPPING_API_PORTAL.md`.
- Skips non-football sport boards by default through `BETTING_SPORT_SCOPE`.
- Converts Norsk Tipping fractional price fields into decimal odds.
- Emits candidates only inside the hard research band, default `1.10-1.35`.
- Includes supported expanded football markets such as goals, corners, cards,
  and player scorer markets.
- Skips events earlier than the live-source cutoff passed by the publisher.
- Leaves `model_probability` and `reference_odds` empty by default. The
  probability model can still rank these candidates from market-implied
  probability plus context and research risk.

`CSV Source`

- Kept for fixtures, manual research, fallback tests, and custom candidate
  files.
- Uses the same football/soccer default scope as live loading unless
  `--sport-scope all` is supplied.
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
- Enforces the requested Norsk Tipping preferred band, default `1.10-1.30`,
  across football/soccer candidates by default.
- Excludes candidates below `1.10` or above `1.35`; `1.30-1.35` is marked as
  fallback-only slack.
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
- Penalizes entertainment/special markets, volatile expanded markets, friendlies,
  injury/rotation/weather risk, and research warnings.
- Applies small confidence adjustments from matched research warnings or
  positive signals.
- Produces explicit risk flags for the report.

`MarketResearchClient`

- Fetches up to 10 configured research sources per run by default.
- Uses `examples/football_research_sources.txt` in scheduled football runs.
- Supports Reddit JSON listing URLs and normal HTML pages.
- Extracts the top configured Reddit posts/items from listing sources.
- Produces page-level positive, warning, and decimal-price signals.
- Keeps fetch failures visible as source-error research notes.

`ResearchAssessment`

- Matches research page text against each candidate's event, market, selection,
  and competition terms.
- Adds evidence notes to the final report.
- Treats social and betting-page findings as weak signals, not formal proof of
  value.

`FootballContextAgent`

- Runs after generic research matching and before final selection.
- Adds a per-candidate checklist for form, injuries/suspensions,
  lineup/rotation, motivation, schedule/travel, weather/venue, and market
  context.
- Uses candidate notes and candidate-specific research matches only.
- Marks missing evidence as `unknown` instead of inventing team context.
- Applies small visible confidence adjustments for positive or warning context,
  with warning categories capped so context cannot overpower the market.

`DailySelectionAgent`

- Applies hard gates for odds band, probability, and confidence.
- Applies edge and expected-value gates only when independent model/reference
  data exists.
- Scores candidates with a success-first probability and context score.
- Selects the top 5 candidates for the daily report by default.
- Fills with best available fallback candidates when fewer than 5 pass every
  strict gate, preferring candidates inside the requested Norsk Tipping odds
  band before any outside-band fallback.
- Allows several preferred markets from the same match when the football board
  has fewer than 5 separate matches.
- Returns `NO BET` only when there are no candidates to rank.

`LearningAgent`

- Reads settled previous pick history from `BETTING_HISTORY_INPUT` when present.
- Applies `BETTING_SETTLEMENTS_JSONL` in memory before learning so same-run
  checked result updates can influence today's ranking.
- Ignores pending, void, and unknown results.
- Builds deterministic football buckets from competition, market type, odds
  range, selection type, and football context warning categories.
- Requires at least 5 similar settled win/loss picks before applying an
  adjustment.
- Caps history confidence movement at +/-3 percentage points and emits a visible
  learning note for every pick.

`PickHistory`

- Runs after deterministic selection and before optional OpenAI rewriting.
- Writes JSON Lines entries for the current ranked picks when
  `BETTING_HISTORY_OUTPUT` is set.
- Reads an optional previous history file from `BETTING_HISTORY_INPUT`.
- Merges reruns by report date, event, market, selection, and start time so the
  same pick is not duplicated.
- Preserves settled `win`, `loss`, or `void` statuses when the same pick is
  regenerated as pending.
- Stores the football context checklist snapshot that existed at pick time.

`ResultSettlement`

- Runs only when `BETTING_SETTLEMENTS_JSONL` points to an explicit settlement
  JSON Lines file.
- Requires exact history keys and a verifiable settlement source for every
  record.
- Accepts `win`, `loss`, `void`, and `unknown`; rejects `pending` settlement
  records.
- Updates only unsettled or unknown history rows.
- Preserves already settled `win`, `loss`, and `void` rows so an unknown check
  cannot erase a verified result.

`OpenAI Agent Workflow`

- Runs only after deterministic filtering and ranking.
- Is enabled in the scheduled GitHub Action when `OPENAI_API_KEY` is configured
  as a repository secret.
- Uses four roles: Explorer, Reviewer, Risk Manager, and Output Writer.
- Passes compact outputs between agents to reduce cost and keep each role
  focused.
- Produces the final user-facing report through the optional `--ai` path.

`Report Renderer`

- Prints the configured football scope and pick target at the top of the report.
- Shows whether pick history is enabled for the run.
- Summarizes source coverage, source errors, missing football context, and
  learning status before the ranked picks.
- Keeps per-pick strict status, football checklist, learning note, research
  notes, and fallback warnings visible.

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

1. Collect current Norsk Tipping football/soccer candidates in the `1.10-1.35`
   research band from live Oddsen data, with `1.10-1.30` preferred.
2. Score candidates from market-implied probability, context confidence,
   research signals, and optional model/reference data.
3. Add risk notes after checking injury, lineup, motivation, market type, and
   market context.
4. Run with research enabled:

```bash
cargo run -- --norsk-tipping-live --date YYYY-MM-DD --sport-scope football --research examples/football_research_sources.txt
```

5. Configure `OPENAI_API_KEY` in GitHub Secrets so the scheduled workflow can run
   the four-agent API review. See `docs/OPENAI_API_SETUP.md`.
6. Publish `today.txt`, the dated report, and the merged `history.jsonl` file to
   the tokenized GitHub Pages path.
7. Optionally apply `BETTING_SETTLEMENTS_JSONL` to update prior pending history
   rows from checked final results.
8. Treat fallback candidates as weaker options and place no bet if the report
   says `NO BET`.
9. For morning delivery, schedule `scripts/daily_betting.sh` with cron and set
   either the SMTP environment variables or the Pushover environment variables.

## Next Extension Points

- Add deeper football-specific probability and context agents.
- Re-check the Norsk Tipping API portal only when authenticated docs,
  permission, rate-limit details, and a stable sportsbook spec are available.
- Add a closing-line-value tracker so the daily process can measure whether the
  selected prices beat the later market.
- Add a small local results database to audit hit rate, expected value, and
  calibration over time.
