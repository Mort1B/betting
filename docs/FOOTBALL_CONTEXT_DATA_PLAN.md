# Football Context Data Plan

Date: 2026-05-20

Scope: close the gap where form, injuries/suspensions, schedule/travel, and
market context stay `unknown` for daily Norsk Tipping football picks.

## Diagnosis

- The current scheduled source file is useful for broad football news and
  betting discussion, but most entries are generic pages.
- The deterministic matcher only trusts a page when it contains candidate terms
  for the match. That is intentional, but it means broad pages often review
  successfully without producing candidate-specific context.
- When a candidate has no model probability and no reference odds, estimated
  probability falls back to Norsk Tipping implied probability. If the football
  checklist is also all `unknown`, the candidate is missing independent support.
- Market context should come from structured bookmaker comparison, not from
  social posts. The Odds API remains the right source for this layer.
- Generic football tips pages must not supply checklist context unless both
  sides of the candidate fixture are found in the source text. Broad league,
  market, or selection words are too weak and can make every pick look the same.

## Immediate Guardrail

Status: implemented in the current working branch.

- Treat `estimated probability == Norsk Tipping implied probability` plus all
  football context buckets unknown as fallback evidence, not a strict
  recommendation.
- Apply a visible confidence penalty and risk flag:
  `market-implied probability lacks independent or context evidence`.
- Map reference-market notes into the `Market context` checklist:
  - `market agreement tight` becomes positive context,
  - `market disagreement high` and `single reference source` become warnings.

## Source Decision

### Flashscore

Use only as a future manual/sidecar source, not as the scheduled default.

Reasoning:
- Flashscore is strong as a consumer match center for scores, stats, standings,
  lineups, and news.
- There is no verified official developer API path in the current workflow.
- Browser scraping would add Playwright/Chromium maintenance, bot-detection risk,
  and likely terms-of-service risk to GitHub Actions.

### API-Football

Best first structured candidate for the missing context layer.

Useful endpoints:
- `/fixtures` by date, team, league, and last/next for matching and schedule.
- `/teams/statistics` for form strings and season home/away performance.
- `/injuries` by fixture, team, league, season, or date for injury/suspension
  context when coverage is available.
- `/leagues` coverage flags so the workflow can say when injury coverage is not
  available instead of treating missing data as clean news.

Fit:
- Good for form, schedule/travel, standings/motivation support, and
  injuries/suspensions.
- Requires a separate API key and strict request budgeting.

### Sportmonks

Second structured candidate if API-Football coverage or pricing is poor.

Useful areas:
- Fixtures by date/range/team, standings, participants, trends, statistics,
  sidelined players, predictions, and prematch news includes.

Fit:
- Good for form, schedule, standings, and potentially sidelined/team-news data.
- Needs a paid/free-trial verification run for the leagues we actually bet.

## Implementation Status

Status on 2026-05-20:

1. Added a separate football-data provider trait in `src/football_data_provider.rs`.
2. Added an env-gated API-Football provider behind
   `BETTING_FOOTBALL_DATA_API_KEY`.
3. The provider fetches same-day fixtures once and matches candidates by
   normalized teams plus kickoff time.
4. The provider fetches fixture injuries/suspensions for at most
   `BETTING_API_FOOTBALL_MAX_FIXTURES` matched fixtures, but only after
   `/leagues` confirms injury coverage for that league-season.
5. The provider checks `/leagues` coverage by league-season and reports
   unavailable or unconfirmed coverage without turning it into positive
   context.
6. The provider fetches `/standings` once per covered league-season and adds
   motivation notes for clear title-race, European/promotion, or relegation
   table positions.
7. The provider fetches recent team fixtures for at most
   `BETTING_API_FOOTBALL_MAX_FORM_TEAMS` teams and turns results/rest days into
   supplied context notes.
8. Provider errors and request counts are shown as football-data provider notes
   without exposing the API key.
9. `today.json` now includes `football_data_provider_notes`.
10. Generic research matching now requires candidate event/team terms and the
    football motivation keyword list no longer treats the bare word `europe` as
    context.

Remaining useful follow-up:

- Add a live manual smoke only after confirming the API-Football key and request
  budget.

## Proposed Implementation

1. Add a separate football-data provider trait, independent from reference odds:
   `FootballContextProvider`.
2. Add an env-gated provider behind `BETTING_FOOTBALL_DATA_API_KEY` and
   `BETTING_FOOTBALL_DATA_PROVIDER=api_football`.
3. Match Norsk Tipping candidates to provider fixtures by normalized home/away
   teams and kickoff time.
4. Populate structured context:
   - form from recent fixtures or team statistics,
   - injuries/suspensions from fixture or team injury data,
   - schedule/travel from last/next fixtures and rest days,
   - motivation from standings, competition phase, and table position,
   - market context from The Odds API reference odds.
5. Add request caps, cache-friendly summaries, and redacted provider notes.
6. Publish context-source metadata in `today.json` so the iPhone Shortcut can
   distinguish `unknown`, `not covered`, and `checked with no issue found`.

## Acceptance Criteria

- A market-implied-only candidate with all context unknown is never a strict
  recommendation.
- Every non-unknown context category includes a source label and freshness note.
- Missing API coverage is reported as missing coverage, not as positive context.
- The Odds API remains capped to at most five bookmaker providers by default.
- Static publishing still fails on partial output or unredacted secrets.
