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
NT_EVENTS_PER_SPORT="${BETTING_NT_EVENTS_PER_SPORT:-35}"
NT_EARLIEST_START="${BETTING_NT_EARLIEST_START:-$(date +%Y-%m-%dT%H:%M)}"
RESEARCH_SOURCES="${BETTING_RESEARCH_SOURCES:-$REPO_DIR/examples/research_sources.txt}"
TODAY="${BETTING_DATE:-$(date +%F)}"
DELIVERY="${BETTING_DELIVERY:-pushover}"

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

SOURCE_ARGS=()
case "$CANDIDATE_SOURCE" in
  csv)
    SOURCE_ARGS=("$INPUT_CSV")
    ;;
  norsk-tipping-live)
    SOURCE_ARGS=(--norsk-tipping-live --nt-events-per-sport "$NT_EVENTS_PER_SPORT" --nt-earliest-start "$NT_EARLIEST_START")
    ;;
  *)
    echo "BETTING_CANDIDATE_SOURCE must be csv or norsk-tipping-live" >&2
    exit 2
    ;;
esac

cargo run -- "${SOURCE_ARGS[@]}" \
  --date "$TODAY" \
  --research "$RESEARCH_SOURCES" \
  "${DELIVERY_ARGS[@]}"
