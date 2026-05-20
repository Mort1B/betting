#!/usr/bin/env bash
set -euo pipefail

DEFAULT_REPO_DIR="/home/morten/Prog/betting"
ENV_FILE="${BETTING_ENV_FILE:-$DEFAULT_REPO_DIR/.env}"
EXTERNAL_BETTING_DELIVERY="${BETTING_DELIVERY:-}"

if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$ENV_FILE"
  set +a
fi

if [[ -n "$EXTERNAL_BETTING_DELIVERY" ]]; then
  BETTING_DELIVERY="$EXTERNAL_BETTING_DELIVERY"
fi

REPO_DIR="${BETTING_REPO_DIR:-$DEFAULT_REPO_DIR}"
INPUT_CSV="${BETTING_INPUT_CSV:-$REPO_DIR/examples/norsk_tipping_candidates.csv}"
CANDIDATE_SOURCE="${BETTING_CANDIDATE_SOURCE:-norsk-tipping-live}"
SPORT_SCOPE="${BETTING_SPORT_SCOPE:-football}"
PICK_COUNT="${BETTING_PICK_COUNT:-5}"
MIN_ODDS="${BETTING_MIN_ODDS:-1.10}"
MAX_ODDS="${BETTING_MAX_ODDS:-1.30}"
MAX_RESEARCH_PAGES="${BETTING_MAX_RESEARCH_PAGES:-13}"
NT_EVENTS_PER_SPORT="${BETTING_NT_EVENTS_PER_SPORT:-35}"
RESEARCH_SOURCES="${BETTING_RESEARCH_SOURCES:-$REPO_DIR/examples/football_research_sources.txt}"
REFERENCE_ODDS_CSV="${BETTING_REFERENCE_ODDS_CSV:-}"
ODDS_API_KEY="${BETTING_ODDS_API_KEY:-}"
ODDS_API_SPORTS="${BETTING_ODDS_API_SPORTS:-soccer_norway_eliteserien,soccer_sweden_allsvenskan,soccer_denmark_superliga,soccer_finland_veikkausliiga,soccer_usa_mls}"
ODDS_API_REGIONS="${BETTING_ODDS_API_REGIONS:-eu}"
ODDS_API_MARKETS="${BETTING_ODDS_API_MARKETS:-h2h}"
ODDS_API_BOOKMAKERS="${BETTING_ODDS_API_BOOKMAKERS:-unibet_se,pinnacle,betfair_ex_eu,betsson,williamhill}"
ODDS_API_COMMENCE_FROM="${BETTING_ODDS_API_COMMENCE_FROM:-}"
ODDS_API_COMMENCE_TO="${BETTING_ODDS_API_COMMENCE_TO:-}"
ODDS_API_EVENT_ODDS_LIMIT="${BETTING_ODDS_API_EVENT_ODDS_LIMIT:-2}"
DELIVERY="${BETTING_DELIVERY:-pushover}"
AI_MAX_OUTPUT_TOKENS="${BETTING_AI_MAX_OUTPUT_TOKENS:-3500}"

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

case "$DELIVERY" in
  email)
    DELIVERY_ARGS=(--send-email)
    ;;
  pushover)
    DELIVERY_ARGS=(--send-pushover)
    ;;
  both)
    DELIVERY_ARGS=(--send-email --send-pushover)
    ;;
  none)
    DELIVERY_ARGS=()
    ;;
  *)
    echo "BETTING_DELIVERY must be email, pushover, both, or none" >&2
    exit 2
    ;;
esac

cd "$REPO_DIR"

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

cargo run -- "${SOURCE_ARGS[@]}" \
  --date "$TODAY" \
  --sport-scope "$SPORT_SCOPE" \
  --pick-count "$PICK_COUNT" \
  --min-odds "$MIN_ODDS" \
  --max-odds "$MAX_ODDS" \
  --research "$RESEARCH_SOURCES" \
  --max-research-pages "$MAX_RESEARCH_PAGES" \
  "${REFERENCE_ARGS[@]}" \
  --ai-max-output-tokens "$AI_MAX_OUTPUT_TOKENS" \
  "${DELIVERY_ARGS[@]}"
