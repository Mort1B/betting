# Daily Betting Agent Workflow

## Objective

Produce a daily top-3 betting report using current Norsk Tipping odds,
probability, context risk, research evidence, and optional comparison signals.

## Pipeline

1. `Input Loader`
   - Reads live Norsk Tipping Oddsen data by default.
   - Keeps CSV input available for fixtures, manual candidates, and fallback
     testing.
   - Requires `norsk_tipping_odds` as the final price.

2. `Reference Odds Enrichment`
   - Optionally applies external comparison prices from `--reference-odds`.
   - Matches by exact candidate id or normalized event, market, and selection.
   - Produces optional `reference_odds` without changing Norsk Tipping as the
     final bet price.

3. `Market Research Client`
   - Fetches configured Reddit JSON and HTML research sources.
   - Produces positive, warning, and price-hint signals.

4. `Deterministic Rust Agents`
   - Filter by date and odds band.
   - Estimate probability from market-implied odds, model probability, and/or
     reference odds.
   - Calculate edge and expected value only when independent inputs exist.
   - Apply context risk and research adjustments.
   - Rank bettable candidates.

5. `Explorer`
   - Reviews the deterministic top-3 for value evidence.

6. `Reviewer`
   - Challenges the ranking and overclaiming.

7. `Risk Manager`
   - Looks for downside risk and no-bet triggers.

8. `Output Writer`
   - Writes the final report for GitHub Pages and iPhone Shortcut consumption.

## Hard Gates

- Norsk Tipping odds must be inside the configured band, default `1.15-1.30`.
- Estimated probability must clear the configured minimum.
- Edge must clear the configured minimum only when independent model/reference
  data exists.
- Confidence must clear the configured minimum.
- The final report still includes the top 3 best available fallback candidates
  when live candidates exist but strict value gates do not pass.

## Publication

The GitHub Action publishes:

```text
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/today.txt
```

The token is stored as `BETTING_REPORT_TOKEN` in GitHub Actions secrets.
