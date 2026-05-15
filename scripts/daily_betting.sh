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

cargo run -- "$INPUT_CSV" \
  --date "$TODAY" \
  --research "$RESEARCH_SOURCES" \
  "${DELIVERY_ARGS[@]}"
