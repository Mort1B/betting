# Football Research Source Review

Date: 2026-05-19

Scope: improve the daily Norsk Tipping football workflow after the late
overnight run. The goal is to keep the report practical: kickoff time visible,
football context focused on useful categories, and research sources grounded in
fetchable data.

## Implemented Now

- Every pick now prints a separate `Kickoff time` line derived from
  `starts_at`, for example `04:00 on 2026-05-20 (Oslo time)`.
- The football context checklist now focuses on:
  - form,
  - injuries/suspensions,
  - motivation,
  - schedule/travel,
  - market context.
- Weather and lineup/rotation are intentionally excluded from the checklist and
  deterministic risk penalties.
- Reddit daily-thread comments remain a first-class research source kind:
  `reddit_thread_search`, but they are not enabled in the scheduled source file
  by default because unauthenticated Reddit JSON/search endpoints currently
  return 403 from scheduled runners.

## Reddit Daily Threads

Live checks on 2026-05-19 showed:

- `r/soccerbetting` exposes current `Daily Picks Thread` posts through Reddit
  JSON search.
- `r/sportsbetting` did not expose a stable daily picks thread through current
  Reddit JSON searches for `daily`, `Daily Picks`, or title-scoped daily search.
- `r/sportsbook` exposes current `Reddit Daily Picks`, `Pick of the Day`, and
  `Soccer Betting and Picks Daily Discussion` threads.

Decision:
- Keep Reddit source support in code for manual or authenticated diagnostics.
- Do not include unauthenticated Reddit URLs in
  `examples/football_research_sources.txt` while they consistently return 403
  from scheduled runners.
- Empty daily-thread searches return no research pages instead of source-error
  noise when Reddit access is available.

## API And Scraping Findings

### Flashscore Scraping

`gustavofariaa/FlashscoreScraping` is useful as prior art, but not the first
scheduled integration target.

Findings:
- The README says Flashscore has no official API.
- The project uses Node and Playwright with Chromium dependencies.
- The exposed data is strongest for historical match results, statistics,
  league data, match date, venue/info, and match statistics.

Decision:
- Do not add a browser scraper to the scheduled GitHub Action yet.
- Keep this as a manual or future sidecar option if paid APIs fail.
- Avoid building directly on scraped Flashscore pages for the daily path unless
  we first accept the maintenance and terms-of-service risk.

### SportDB.dev

SportDB.dev is worth testing for fixtures, live scores, standings, player data,
match stats, and lineups. The public docs mention REST, MCP, live scores,
fixtures, standings, match details, lineups, and stats.

Fit:
- Good for fixtures, schedule context, standings, and form-like context.
- Not enough public evidence that it covers bookmaker odds or injury reports.
- Lineups are available, but lineup/rotation is no longer a required checklist
  category.

Decision:
- Consider only if we need structured fixtures, standings, or team/player stats.
- Do not choose it as the primary market-context provider.

### Odds APIs

Market context needs multiple bookmaker prices. That is better handled by an
odds API than by scraping Flashscore.

Candidates:
- `the-odds-api.com`: documented JSON feed, soccer coverage, bookmaker odds,
  event start time, participants, and bookmaker-region odds in one request.
- `theoddsapi.com`: normalized multi-book odds, 50-bookmaker positioning, soccer
  coverage, and paid plans from low monthly pricing.

Decision:
- Next paid-source experiment should be an odds API, not Flashscore scraping.
- Start with the cheaper documented odds provider that can return soccer events,
  start time, participants, and multi-book decimal odds for European football.
- Store API-derived market prices through the existing reference-odds enrichment
  path so Norsk Tipping remains the final bet price.

### RapidAPI Flashscore4

The public RapidAPI page was too thin to verify endpoint coverage without
signing in and testing a key.

Decision:
- Only test it after deciding that Flashscore-derived data is worth paying for.
- Do not commit the workflow to a RapidAPI dependency until endpoint coverage,
  request limits, response shape, and terms are verified.

## Proposed Next Implementation

Status on 2026-05-20:

1. Added a generic reference-odds provider adapter that writes the existing
   `ReferenceOddsRow` shape in memory.
2. Added an env-gated The Odds API implementation wired through
   `BETTING_ODDS_API_KEY`.
3. The provider matches h2h/main-market football events to Norsk Tipping
   candidates by normalized teams, kickoff time, market shape, and selection.
4. The default bookmaker set is capped to five explicit keys:
   `unibet_se,pinnacle,betfair_ex_eu,betsson,williamhill`.
5. Provider HTTP, JSON, empty-response, and no-match outcomes are appended as
   reference-provider notes instead of failing the report.
6. Added a local The Odds API h2h JSON fixture test before enabling the paid API
   path in scheduled GitHub Actions.
7. Added a report run-summary line for provider request count, successful
   request count, returned events, matched reference rows, matched candidates,
   and bookmaker-key count without exposing the API key.
8. Added opt-in The Odds API `totals` mapping for Norsk Tipping over/under
   selections. Scheduled defaults remain `h2h` only to conserve credits.
9. Added opt-in The Odds API `double_chance` mapping through the event-level
   odds endpoint, capped by `BETTING_ODDS_API_EVENT_ODDS_LIMIT`. Scheduled
   defaults remain `h2h` only.

Remaining useful follow-up:

1. Run one live, manually triggered provider smoke with
   `BETTING_ODDS_API_MARKETS=h2h,double_chance` and a low
   `BETTING_ODDS_API_EVENT_ODDS_LIMIT` only when spending those credits is
   acceptable.

## Closeout Hardening Plan

Status on 2026-05-20:

1. Tighten AI role instructions so Explorer acts as a football market analyst
   and Reviewer acts as a football betting skeptic, while both remain restricted
   to supplied evidence.
2. Add Odds API credit telemetry from response headers to the provider run
   summary.
3. Add source freshness notes for provider odds updates.
4. Add reference-market range, average, source count, and disagreement notes to
   candidate context.
5. Validate static report artifacts before publishing so partial JSON/text or
   unredacted secret-looking values fail the workflow.
6. Document `today.json` as the preferred iPhone Shortcut input.

## Follow-Up Context Data Plan

The next gap is structured candidate-specific context. Broad football pages can
be fetched successfully while still leaving form, injuries/suspensions,
schedule/travel, and market context unknown for the actual Norsk Tipping match.

See `docs/FOOTBALL_CONTEXT_DATA_PLAN.md` for the source decision and
implementation plan. Short version: keep The Odds API for market context, treat
Flashscore as a future manual/sidecar option rather than the scheduled default,
and verify API-Football first for structured form, schedule, standings, and
injury/suspension context.
