#!/usr/bin/env bash
set -euo pipefail

DEFAULT_REPO_DIR="/home/morten/Prog/betting"
REPO_DIR="${BETTING_REPO_DIR:-$DEFAULT_REPO_DIR}"
PUBLIC_DIR="${BETTING_PUBLIC_DIR:-$REPO_DIR/public}"
INPUT_CSV="${BETTING_INPUT_CSV:-$REPO_DIR/examples/norsk_tipping_candidates.csv}"
RESEARCH_SOURCES="${BETTING_RESEARCH_SOURCES:-$REPO_DIR/examples/football_research_sources.txt}"
REFERENCE_ODDS_CSV="${BETTING_REFERENCE_ODDS_CSV:-}"
ODDS_API_KEY="${BETTING_ODDS_API_KEY:-}"
ODDS_API_SPORTS="${BETTING_ODDS_API_SPORTS:-auto}"
ODDS_API_REGIONS="${BETTING_ODDS_API_REGIONS:-eu}"
ODDS_API_MARKETS="${BETTING_ODDS_API_MARKETS:-h2h}"
ODDS_API_BOOKMAKERS="${BETTING_ODDS_API_BOOKMAKERS:-unibet_se,pinnacle,betfair_ex_eu,betsson,williamhill}"
ODDS_API_COMMENCE_FROM="${BETTING_ODDS_API_COMMENCE_FROM:-}"
ODDS_API_COMMENCE_TO="${BETTING_ODDS_API_COMMENCE_TO:-}"
ODDS_API_EVENT_ODDS_LIMIT="${BETTING_ODDS_API_EVENT_ODDS_LIMIT:-2}"
FOOTBALL_DATA_PROVIDER="${BETTING_FOOTBALL_DATA_PROVIDER:-api_football}"
FOOTBALL_DATA_API_KEY="${BETTING_FOOTBALL_DATA_API_KEY:-}"
API_FOOTBALL_BASE_URL="${BETTING_API_FOOTBALL_BASE_URL:-}"
API_FOOTBALL_TIMEZONE="${BETTING_API_FOOTBALL_TIMEZONE:-Europe/Oslo}"
API_FOOTBALL_MAX_FIXTURES="${BETTING_API_FOOTBALL_MAX_FIXTURES:-5}"
API_FOOTBALL_MAX_FORM_TEAMS="${BETTING_API_FOOTBALL_MAX_FORM_TEAMS:-10}"
REPORT_TOKEN="${BETTING_REPORT_TOKEN:-${REPORT_TOKEN:-}}"
ENABLE_AI="${BETTING_ENABLE_AI:-false}"
OPENAI_MODEL="${BETTING_OPENAI_MODEL:-gpt-5.5}"
AI_MAX_OUTPUT_TOKENS="${BETTING_AI_MAX_OUTPUT_TOKENS:-3500}"
CANDIDATE_SOURCE="${BETTING_CANDIDATE_SOURCE:-norsk-tipping-live}"
SPORT_SCOPE="${BETTING_SPORT_SCOPE:-football}"
PICK_COUNT="${BETTING_PICK_COUNT:-5}"
MIN_ODDS="${BETTING_MIN_ODDS:-1.10}"
MAX_ODDS="${BETTING_MAX_ODDS:-1.30}"
MAX_RESEARCH_PAGES="${BETTING_MAX_RESEARCH_PAGES:-13}"
NT_EVENTS_PER_SPORT="${BETTING_NT_EVENTS_PER_SPORT:-35}"
DELIVERY="${BETTING_DELIVERY:-none}"

default_report_date() {
  if [[ -n "${BETTING_DATE:-}" ]]; then
    printf '%s\n' "$BETTING_DATE"
    return
  fi

  local hour
  hour="$(TZ=Europe/Oslo date +%H)"
  if ((10#$hour < 5)); then
    TZ=Europe/Oslo date -d "yesterday" +%F
  else
    TZ=Europe/Oslo date +%F
  fi
}

TODAY="$(default_report_date)"
NT_EARLIEST_START="${BETTING_NT_EARLIEST_START:-${TODAY}T16:00}"
NT_LATEST_START="${BETTING_NT_LATEST_START:-$(TZ=Europe/Oslo date -d "$TODAY +1 day" +%Y-%m-%d)T05:00}"

if [[ -z "$REPORT_TOKEN" ]]; then
  echo "BETTING_REPORT_TOKEN or REPORT_TOKEN is required" >&2
  exit 2
fi

REPORT_DIR="$PUBLIC_DIR/$REPORT_TOKEN"
TODAY_REPORT="$REPORT_DIR/today.txt"
DATED_REPORT="$REPORT_DIR/$TODAY.txt"
TODAY_JSON="$REPORT_DIR/today.json"
DATED_JSON="$REPORT_DIR/$TODAY.json"
HISTORY_REPORT="$REPORT_DIR/history.jsonl"
HISTORY_INPUT="$(mktemp "${TMPDIR:-/tmp}/betting-history.XXXXXX.jsonl")"
HISTORY_URL="${BETTING_HISTORY_URL:-https://mort1b.github.io/betting/$REPORT_TOKEN/history.jsonl}"
HISTORY_MAX_BYTES="${BETTING_HISTORY_MAX_BYTES:-5000000}"

mkdir -p "$REPORT_DIR"
cd "$REPO_DIR"

AI_ARGS=()
if [[ "$ENABLE_AI" == "true" || "$ENABLE_AI" == "1" ]]; then
  AI_ARGS=(--ai --openai-model "$OPENAI_MODEL" --ai-max-output-tokens "$AI_MAX_OUTPUT_TOKENS")
fi

case "$DELIVERY" in
  pushover)
    DELIVERY_ARGS=(--send-pushover)
    ;;
  none)
    DELIVERY_ARGS=()
    ;;
  *)
    echo "BETTING_DELIVERY must be pushover or none for static publishing" >&2
    exit 2
    ;;
esac

REFERENCE_ARGS=()
if [[ -n "$REFERENCE_ODDS_CSV" ]]; then
  if [[ ! -f "$REFERENCE_ODDS_CSV" ]]; then
    echo "BETTING_REFERENCE_ODDS_CSV does not exist: $REFERENCE_ODDS_CSV" >&2
    exit 2
  fi
  REFERENCE_ARGS=(--reference-odds "$REFERENCE_ODDS_CSV")
elif [[ -f "$REPO_DIR/reference_odds.csv" ]]; then
  REFERENCE_ARGS=(--reference-odds "$REPO_DIR/reference_odds.csv")
fi

if [[ -n "$ODDS_API_KEY" ]]; then
  REFERENCE_ARGS+=(
    --odds-api-key "$ODDS_API_KEY"
    --odds-api-sports "$ODDS_API_SPORTS"
    --odds-api-regions "$ODDS_API_REGIONS"
    --odds-api-markets "$ODDS_API_MARKETS"
    --odds-api-event-odds-limit "$ODDS_API_EVENT_ODDS_LIMIT"
  )
  if [[ -n "$ODDS_API_BOOKMAKERS" ]]; then
    REFERENCE_ARGS+=(--odds-api-bookmakers "$ODDS_API_BOOKMAKERS")
  fi
  if [[ -n "$ODDS_API_COMMENCE_FROM" ]]; then
    REFERENCE_ARGS+=(--odds-api-commence-from "$ODDS_API_COMMENCE_FROM")
  fi
  if [[ -n "$ODDS_API_COMMENCE_TO" ]]; then
    REFERENCE_ARGS+=(--odds-api-commence-to "$ODDS_API_COMMENCE_TO")
  fi
fi

FOOTBALL_DATA_ARGS=()
if [[ -n "$FOOTBALL_DATA_API_KEY" ]]; then
  if [[ "$FOOTBALL_DATA_PROVIDER" != "api_football" ]]; then
    echo "BETTING_FOOTBALL_DATA_PROVIDER must be api_football" >&2
    exit 2
  fi
  FOOTBALL_DATA_ARGS=(
    --api-football-key "$FOOTBALL_DATA_API_KEY"
    --api-football-timezone "$API_FOOTBALL_TIMEZONE"
    --api-football-max-fixtures "$API_FOOTBALL_MAX_FIXTURES"
    --api-football-max-form-teams "$API_FOOTBALL_MAX_FORM_TEAMS"
  )
  if [[ -n "$API_FOOTBALL_BASE_URL" ]]; then
    FOOTBALL_DATA_ARGS+=(--api-football-base-url "$API_FOOTBALL_BASE_URL")
  fi
fi

SOURCE_ARGS=()
case "$CANDIDATE_SOURCE" in
  csv)
    SOURCE_ARGS=("$INPUT_CSV")
    ;;
  norsk-tipping-live)
    SOURCE_ARGS=(--norsk-tipping-live --nt-events-per-sport "$NT_EVENTS_PER_SPORT" --nt-earliest-start "$NT_EARLIEST_START" --nt-latest-start "$NT_LATEST_START")
    ;;
  *)
    echo "BETTING_CANDIDATE_SOURCE must be csv or norsk-tipping-live" >&2
    exit 2
    ;;
esac

if command -v curl >/dev/null 2>&1; then
  if ! curl -fsSL --max-time 10 --max-filesize "$HISTORY_MAX_BYTES" "$HISTORY_URL" -o "$HISTORY_INPUT" 2>/dev/null; then
    : > "$HISTORY_INPUT"
  fi
else
  : > "$HISTORY_INPUT"
fi

BETTING_HISTORY_INPUT="$HISTORY_INPUT" BETTING_HISTORY_OUTPUT="$HISTORY_REPORT" BETTING_JSON_OUTPUT="$TODAY_JSON" cargo run -- "${SOURCE_ARGS[@]}" \
  --date "$TODAY" \
  --sport-scope "$SPORT_SCOPE" \
  --pick-count "$PICK_COUNT" \
  --min-odds "$MIN_ODDS" \
  --max-odds "$MAX_ODDS" \
  --research "$RESEARCH_SOURCES" \
  --max-research-pages "$MAX_RESEARCH_PAGES" \
  "${REFERENCE_ARGS[@]}" \
  "${FOOTBALL_DATA_ARGS[@]}" \
  "${AI_ARGS[@]}" \
  "${DELIVERY_ARGS[@]}" \
  > "$TODAY_REPORT"

cp "$TODAY_REPORT" "$DATED_REPORT"
cp "$TODAY_JSON" "$DATED_JSON"

cat > "$PUBLIC_DIR/index.html" <<'HTML'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="robots" content="noindex,nofollow">
    <title>Betting Report</title>
  </head>
  <body>
    <h1>Betting Report</h1>
    <p>Use the private report URL configured in your iPhone Shortcut.</p>
  </body>
</html>
HTML

cat > "$REPORT_DIR/today.html" <<'HTML'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="robots" content="noindex,nofollow">
    <title>Daily Betting Report</title>
    <style>
      :root {
        color-scheme: light dark;
        font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      }
      body {
        margin: 0;
        padding: 20px;
        line-height: 1.45;
      }
      header {
        display: flex;
        flex-wrap: wrap;
        gap: 12px;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 16px;
      }
      h1 {
        margin: 0;
        font-size: 1.25rem;
      }
      a {
        margin-right: 12px;
      }
      pre {
        max-width: 100%;
        overflow-x: auto;
        white-space: pre-wrap;
        overflow-wrap: anywhere;
        font: 0.9rem/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      }
    </style>
  </head>
  <body>
    <header>
      <h1>Daily Betting Report</h1>
      <nav>
        <a href="./today.txt">Text</a>
        <a href="./today.json">JSON</a>
        <a href="./history.jsonl">History</a>
      </nav>
    </header>
    <pre id="report">Loading report...</pre>
    <script>
      fetch("./today.txt", { cache: "no-store" })
        .then((response) => {
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          return response.text();
        })
        .then((text) => {
          document.getElementById("report").textContent = text;
        })
        .catch((error) => {
          document.getElementById("report").textContent =
            `Could not load today.txt: ${error.message}`;
        });
    </script>
  </body>
</html>
HTML

cat > "$REPORT_DIR/index.html" <<HTML
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="robots" content="noindex,nofollow">
    <meta http-equiv="refresh" content="0; url=./today.html">
    <title>Daily Betting Report</title>
  </head>
  <body>
    <p><a href="./today.html">Open today's betting report</a></p>
    <p><a href="./today.json">Open today's betting JSON</a></p>
  </body>
</html>
HTML

bash "$REPO_DIR/scripts/validate_static_report.sh" "$REPORT_DIR"
