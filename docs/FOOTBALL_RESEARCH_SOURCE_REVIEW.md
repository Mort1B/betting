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
- Reddit daily-thread comments are now a first-class research source kind:
  `reddit_thread_search`.
- The scheduled football source file includes:
  - `r/soccerbetting` daily picks thread comments,
  - a guarded `r/sportsbetting` daily picks thread search,
  - `r/sportsbook` soccer daily discussion comments as the practical fallback.

## Reddit Daily Threads

Live checks on 2026-05-19 showed:

- `r/soccerbetting` exposes current `Daily Picks Thread` posts through Reddit
  JSON search.
- `r/sportsbetting` did not expose a stable daily picks thread through current
  Reddit JSON searches for `daily`, `Daily Picks`, or title-scoped daily search.
- `r/sportsbook` exposes current `Reddit Daily Picks`, `Pick of the Day`, and
  `Soccer Betting and Picks Daily Discussion` threads.

Decision:
- Use `r/soccerbetting` directly.
- Keep a guarded `r/sportsbetting` search in the scheduled source file. Empty
  daily-thread searches return no research pages instead of source-error noise.
- Use `r/sportsbook` soccer daily discussion as the stable daily-thread fallback
  for general sports-betting Reddit context.
- Fetch Reddit with curl first from Rust because live testing showed Reddit can
  return empty or forbidden responses to the `reqwest` client while curl returns
  the expected JSON listing.

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

1. Add a generic `reference_odds_provider` adapter that can write the existing
   `ReferenceOddsRow` shape in memory.
2. Add an implementation behind env-gated credentials, for example
   `BETTING_ODDS_API_KEY`.
3. Match API events to Norsk Tipping candidates by normalized teams, start time,
   market, and selection.
4. Keep failures visible as source-error or reference-odds notes, not as hard
   report failures.
5. Add tests with local fixture JSON for odds-provider matching before using any
   paid API in GitHub Actions.
