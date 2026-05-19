#!/usr/bin/env bash
set -euo pipefail

DEFAULT_REPO_DIR="/home/morten/Prog/betting"
REPO_DIR="${BETTING_REPO_DIR:-$DEFAULT_REPO_DIR}"
PUBLIC_DIR="${BETTING_PUBLIC_DIR:-$REPO_DIR/public}"
INPUT_CSV="${BETTING_INPUT_CSV:-$REPO_DIR/examples/norsk_tipping_candidates.csv}"
RESEARCH_SOURCES="${BETTING_RESEARCH_SOURCES:-$REPO_DIR/examples/football_research_sources.txt}"
REFERENCE_ODDS_CSV="${BETTING_REFERENCE_ODDS_CSV:-}"
REPORT_TOKEN="${BETTING_REPORT_TOKEN:-${REPORT_TOKEN:-}}"
ENABLE_AI="${BETTING_ENABLE_AI:-false}"
OPENAI_MODEL="${BETTING_OPENAI_MODEL:-gpt-5.5}"
CANDIDATE_SOURCE="${BETTING_CANDIDATE_SOURCE:-norsk-tipping-live}"
SPORT_SCOPE="${BETTING_SPORT_SCOPE:-football}"
PICK_COUNT="${BETTING_PICK_COUNT:-5}"
MIN_ODDS="${BETTING_MIN_ODDS:-1.10}"
MAX_ODDS="${BETTING_MAX_ODDS:-1.30}"
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
HISTORY_REPORT="$REPORT_DIR/history.jsonl"
HISTORY_INPUT="$(mktemp "${TMPDIR:-/tmp}/betting-history.XXXXXX.jsonl")"
HISTORY_URL="${BETTING_HISTORY_URL:-https://mort1b.github.io/betting/$REPORT_TOKEN/history.jsonl}"

mkdir -p "$REPORT_DIR"
cd "$REPO_DIR"

AI_ARGS=()
if [[ "$ENABLE_AI" == "true" || "$ENABLE_AI" == "1" ]]; then
  AI_ARGS=(--ai --openai-model "$OPENAI_MODEL")
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
  if ! curl -fsSL --max-time 10 "$HISTORY_URL" -o "$HISTORY_INPUT" 2>/dev/null; then
    : > "$HISTORY_INPUT"
  fi
else
  : > "$HISTORY_INPUT"
fi

BETTING_HISTORY_INPUT="$HISTORY_INPUT" BETTING_HISTORY_OUTPUT="$HISTORY_REPORT" cargo run -- "${SOURCE_ARGS[@]}" \
  --date "$TODAY" \
  --sport-scope "$SPORT_SCOPE" \
  --pick-count "$PICK_COUNT" \
  --min-odds "$MIN_ODDS" \
  --max-odds "$MAX_ODDS" \
  --research "$RESEARCH_SOURCES" \
  "${REFERENCE_ARGS[@]}" \
  "${AI_ARGS[@]}" \
  "${DELIVERY_ARGS[@]}" \
  > "$TODAY_REPORT"

cp "$TODAY_REPORT" "$DATED_REPORT"

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

cat > "$REPORT_DIR/index.html" <<HTML
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="robots" content="noindex,nofollow">
    <meta http-equiv="refresh" content="0; url=./today.txt">
    <title>Daily Betting Report</title>
  </head>
  <body>
    <p><a href="./today.txt">Open today's betting report</a></p>
  </body>
</html>
HTML
