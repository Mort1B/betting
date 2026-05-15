#!/usr/bin/env bash
set -euo pipefail

DEFAULT_REPO_DIR="/home/morten/Prog/betting"
REPO_DIR="${BETTING_REPO_DIR:-$DEFAULT_REPO_DIR}"
PUBLIC_DIR="${BETTING_PUBLIC_DIR:-$REPO_DIR/public}"
INPUT_CSV="${BETTING_INPUT_CSV:-$REPO_DIR/examples/norsk_tipping_candidates.csv}"
RESEARCH_SOURCES="${BETTING_RESEARCH_SOURCES:-$REPO_DIR/examples/research_sources.txt}"
TODAY="${BETTING_DATE:-$(TZ=Europe/Oslo date +%F)}"
REPORT_TOKEN="${BETTING_REPORT_TOKEN:-${REPORT_TOKEN:-}}"

if [[ -z "$REPORT_TOKEN" ]]; then
  echo "BETTING_REPORT_TOKEN or REPORT_TOKEN is required" >&2
  exit 2
fi

REPORT_DIR="$PUBLIC_DIR/$REPORT_TOKEN"
TODAY_REPORT="$REPORT_DIR/today.txt"
DATED_REPORT="$REPORT_DIR/$TODAY.txt"

mkdir -p "$REPORT_DIR"
cd "$REPO_DIR"

cargo run -- "$INPUT_CSV" \
  --date "$TODAY" \
  --research "$RESEARCH_SOURCES" \
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
