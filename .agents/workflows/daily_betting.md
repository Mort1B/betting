# Daily Betting Agent Workflow

## Objective

Produce a daily top-3 betting report using current Norsk Tipping odds, independent
comparison signals, research evidence, and risk review.

## Pipeline

1. `Input Loader`
   - Reads live Norsk Tipping Oddsen data by default.
   - Keeps CSV input available for fixtures, manual candidates, and fallback
     testing.
   - Requires `norsk_tipping_odds` as the final price.

2. `Market Research Client`
   - Fetches configured Reddit JSON and HTML research sources.
   - Produces positive, warning, and price-hint signals.

3. `Deterministic Rust Agents`
   - Filter by date and odds band.
   - Estimate probability from model probability and/or reference odds.
   - Calculate edge and expected value.
   - Apply risk and research adjustments.
   - Rank bettable candidates.

4. `Explorer`
   - Reviews the deterministic top-3 for value evidence.

5. `Reviewer`
   - Challenges the ranking and overclaiming.

6. `Risk Manager`
   - Looks for downside risk and no-bet triggers.

7. `Output Writer`
   - Writes the final report for GitHub Pages and iPhone Shortcut consumption.

## Hard Gates

- Norsk Tipping odds must be inside the configured band, default `1.15-1.30`.
- Estimated probability must clear the configured minimum.
- Edge must clear the configured minimum.
- Confidence must clear the configured minimum.
- Missing independent signal rejects a candidate.
- The final report still includes the top 3 best available fallback candidates
  when live candidates exist but strict value gates do not pass.

## Publication

The GitHub Action publishes:

```text
https://mort1b.github.io/betting/<BETTING_REPORT_TOKEN>/today.txt
```

The token is stored as `BETTING_REPORT_TOKEN` in GitHub Actions secrets.
