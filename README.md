# Betting Daily Agent

Rust command line tool for choosing one daily Norsk Tipping bet candidate in the
requested 1.15-1.30 odds band.

The system is intentionally conservative: it outputs the top 3 bettable
candidates with explanations, but if no candidate has a positive edge, enough
estimated win probability, and enough confidence, the answer is `NO BET`. That
is still part of the daily process.

## Quick Start

```bash
cargo run -- examples/norsk_tipping_candidates.csv --date 2026-05-15
```

With research sources:

```bash
cargo run -- examples/norsk_tipping_candidates.csv \
  --date 2026-05-15 \
  --research examples/research_sources.txt
```

Input is a CSV of candidates copied or exported from Norsk Tipping plus your own
probability signals. Norsk Tipping odds are the price that matters for the bet,
but they are not enough on their own to prove value.

## CSV Contract

Required columns:

- `id`
- `sport`
- `competition`
- `event`
- `market`
- `selection`
- `norsk_tipping_odds`
- `starts_at`

Optional but important columns:

- `model_probability`: your estimated probability from a model, research sheet,
  or manual assessment.
- `reference_odds`: a comparable market price from another source. This is used
  as an independent probability signal.
- `confidence`: 0.0-1.0 confidence score after checking team news, lineup,
  injury, motivation, and market stability.
- `notes`: free-text context. Risk words such as `injury`, `rotation`, `weather`,
  `derby`, and `cup` reduce confidence.

## Default Gates

- Norsk Tipping odds must be between `1.15` and `1.30`.
- Estimated probability must be at least `79%`.
- Edge versus Norsk Tipping implied probability must be at least `1.5`
  percentage points.
- Confidence must be at least `65%`.
- Expected value must be non-negative.

All thresholds can be changed with CLI flags. Run with `--help` to see the
available options.

## Research Sources

Research is configured with a `name|kind|url` file. The default example is
`examples/research_sources.txt`, which includes Reddit JSON listing sources and
a normal HTML page.

Supported kinds:

- `reddit_json`: reads top posts from Reddit JSON listing URLs such as
  `/r/soccerbetting/top.json?t=day&limit=10`.
- `html`: fetches a normal web page and extracts text from the HTML body.

The program reviews up to `--max-research-pages 10` configured sources and up to
`--max-research-items 10` posts/items from listing sources. It looks for team,
market, selection, value, warning, and price-hint terms and adds those findings
to the final recommendation. Social posts and betting pages are treated as
research signals, not proof.

## Morning Delivery

Recommended setup: GitHub Actions publishes the report to GitHub Pages, and an
iPhone Shortcut fetches that URL every morning. This avoids Gmail credentials,
Pushover, a VPS, and DNS. See `docs/GITHUB_PAGES_SHORTCUT.md`.

Email and iPhone push delivery are supported:

```bash
cargo run -- examples/norsk_tipping_candidates.csv \
  --date 2026-05-15 \
  --research examples/research_sources.txt \
  --send-email
```

```bash
cargo run -- examples/norsk_tipping_candidates.csv \
  --date 2026-05-15 \
  --research examples/research_sources.txt \
  --send-pushover
```

See `docs/MORNING_DELIVERY.md` for SMTP/Pushover environment variables and the
cron setup.

Local settings live in `.env`, which is ignored by git because it can contain
delivery credentials. `.env.example` is the shareable template.

## Norsk Tipping Notes

The architecture uses `Oddsen`, not `Langoddsen`: Norsk Tipping's own site says
Langoddsen is no longer offered and points players to Oddsen/Oddsbomben instead.
Norsk Tipping also states that odds express probability, are set from available
information such as statistics, form, lineup, and home advantage, and may change
before a bet is submitted. Run the tool close to when you place the bet and use
the current Norsk Tipping price.

Sources:

- https://www.norsk-tipping.no/sport/oddsen/slik-spiller-du
- https://www.norsk-tipping.no/kundeservice/velge-spill/oddsen/hva-er-odds
- https://www.norsk-tipping.no/sport/langoddsen

## Responsible Use

This tool is decision support, not a guarantee. It should never increase stake
size after losses, chase action, or force a bet when the daily slate is poor.
