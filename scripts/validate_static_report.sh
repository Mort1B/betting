#!/usr/bin/env bash
set -euo pipefail

REPORT_DIR="${1:-${REPORT_DIR:-}}"

if [[ -z "$REPORT_DIR" ]]; then
  echo "usage: validate_static_report.sh REPORT_DIR" >&2
  exit 2
fi

if [[ ! -d "$REPORT_DIR" ]]; then
  echo "report directory does not exist: $REPORT_DIR" >&2
  exit 2
fi

for file in today.txt today.json today.html history.jsonl; do
  if [[ ! -s "$REPORT_DIR/$file" ]]; then
    echo "missing or empty report artifact: $REPORT_DIR/$file" >&2
    exit 1
  fi
done

jq -e '
  .schema_version == 1
  and (.reports.final_text | type == "string")
  and (.reports.deterministic_text | type == "string")
  and (.decision.picks | type == "array")
  and (
    if .decision.kind == "no_bet" then true
    else
      (.decision.picks | length) > 0
      and (.reports.final_text as $text
        | [.decision.picks[] | "#\(.rank) \(.candidate.event)"] as $headings
        | all($headings[]; $text | contains(.)))
    end
  )
' "$REPORT_DIR/today.json" >/dev/null

if grep -R "apiKey=" "$REPORT_DIR" | grep -v "apiKey=<redacted>" >/dev/null; then
  echo "unredacted apiKey value found in published report artifacts" >&2
  exit 1
fi

if grep -R -E "OPENAI_API_KEY|BETTING_ODDS_API_KEY|sk-[A-Za-z0-9]" "$REPORT_DIR" >/dev/null; then
  echo "secret-looking token or secret environment variable name found in report artifacts" >&2
  exit 1
fi

echo "static report validation passed"
