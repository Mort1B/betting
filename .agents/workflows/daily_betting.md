# Daily Betting Agent Workflow

## Objective

Produce a daily football/soccer top-5 betting report using current Norsk
Tipping odds, probability, context risk, research evidence, and optional
comparison signals.

## Pipeline

1. `Input Loader`
   - Reads live Norsk Tipping Oddsen football/soccer data by default.
   - Keeps CSV input available for fixtures, manual candidates, and fallback
     testing.
   - Requires `norsk_tipping_odds` as the final price.

2. `Reference Odds Enrichment`
   - Optionally applies external comparison prices from `--reference-odds`.
   - Optionally applies env-gated The Odds API h2h/main-market rows.
   - Applies The Odds API totals rows only when `BETTING_ODDS_API_MARKETS`
     explicitly includes `totals`.
   - Applies The Odds API double-chance rows only when
     `BETTING_ODDS_API_MARKETS` explicitly includes `double_chance`, capped by
     `BETTING_ODDS_API_EVENT_ODDS_LIMIT`.
   - Caps The Odds API bookmaker keys at 5, defaulting to Unibet SE, Pinnacle,
     Betfair Exchange EU, Betsson, and William Hill.
   - Matches by exact candidate id or normalized event, market, selection, teams,
     and kickoff time.
   - Prints run-level provider request and match counts without exposing the API
     key.
   - Produces optional `reference_odds` without changing Norsk Tipping as the
     final bet price.

3. `Football Data Enrichment`
   - Optionally applies API-Football fixture, coverage, injury/suspension,
     recent form, standings motivation, and rest-day context when
     `BETTING_FOOTBALL_DATA_API_KEY` is configured.
   - Caps matched fixture and team-form requests to keep scheduled API usage
     bounded.
   - Treats unavailable coverage as missing context, not as positive evidence.
   - Adds supplied context into candidate notes before the football checklist is
     assessed.

4. `Market Research Client`
   - Fetches configured football HTML research sources by default.
   - Keeps Reddit JSON source support available for manual or authenticated
     diagnostics, but scheduled defaults exclude unauthenticated Reddit URLs
     while they return 403.
   - Uses `examples/football_research_sources.txt` by default.
   - Produces positive, warning, and price-hint signals.

5. `Deterministic Rust Agents`
   - Filter by date and the hard research odds band.
   - Keep `1.10-1.30` as the preferred odds band and `1.30-1.35` as fallback
     slack only.
   - Allow multiple ranked markets from the same match when fewer than 5
     separate football matches are available.
   - Estimate probability from market-implied odds, model probability, and/or
     reference odds.
   - Calculate edge and expected value only when independent inputs exist.
   - Apply context risk, research adjustments, and football context categories.
   - Apply capped learning adjustments from settled historical football buckets
     when enough similar picks exist.
   - Rank bettable candidates.

6. `Learning Note`
   - Every pick reports whether history was unavailable, insufficient, or
     adjusted confidence from a settled bucket.
   - Pending, void, and unknown results are ignored for learning.

7. `Explorer`
   - Reviews the deterministic top-5 for value evidence.

8. `Reviewer`
   - Challenges the ranking and overclaiming.

9. `Risk Manager`
   - Looks for downside risk and no-bet triggers.

10. `Output Writer`
   - Writes the final report for GitHub Pages and iPhone Shortcut consumption.
   - Must preserve every ranked candidate heading so the publisher can reject
     partial AI output.

11. `Pick History`
   - Publishes `history.jsonl` beside `today.txt`.
   - Fetches the previous Pages history before publishing when available.
   - Merges current picks idempotently and preserves settled results.

12. `Result Settlement`
   - Runs only from explicit `BETTING_SETTLEMENTS_JSONL` JSON Lines records.
   - Requires exact history keys and a settlement source.
   - Supports `win`, `loss`, `void`, and `unknown`.
   - Does not infer results from free-text research pages.

## Hard Gates

- Norsk Tipping odds must be at least `1.10` and at most `1.35` before any
  candidate-specific ranking or research assessment.
- The preferred strict band is `1.10-1.30`; `1.30-1.35` can only be fallback.
- Estimated probability must clear the configured minimum.
- Edge must clear the configured minimum only when independent model/reference
  data exists.
- Confidence must clear the configured minimum.
- The final report still includes the top 5 best available fallback candidates
  when live candidates exist but strict value gates do not pass.

## Publication

The GitHub Action publishes:

```text
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/today.txt
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/today.json
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/history.jsonl
```

The token is stored as `BETTING_REPORT_TOKEN` in GitHub Actions secrets.
Use `today.json` for iPhone Shortcut parsing and `today.html` for the full
readable report.
