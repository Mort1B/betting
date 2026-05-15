# OpenAI Agent Workflow

The daily report can run through four OpenAI-backed agents after the deterministic
Rust filters have ranked the candidates.

This is intentionally optimized:

1. Rust filters all candidates first.
2. Rust builds a compact top-3 deterministic report.
3. The AI agents only receive that compact report and prior agent outputs.
4. Each agent has a narrow role and a small output budget.

## Agents

`Explorer`

- Looks for the strongest value evidence.
- Checks Norsk Tipping odds against reference odds, model probability, EV, and
  research matches.
- Calls out missing comparison data.

`Reviewer`

- Challenges the ranking and the Explorer output.
- Looks for likely-but-not-value cases, weak evidence, and overclaiming.
- Approves, questions, or rejects each candidate in concise terms.

`Risk Manager`

- Identifies downside risks, missing data, and no-bet triggers.
- Downgrades candidates when risk is not reflected in the deterministic score.
- Never treats a bet as guaranteed.

`Output Writer`

- Writes the final top-3 user-facing report.
- Includes sport, competition, event, market, selection, Norsk Tipping odds,
  reference-market comparison, rationale, risks, and confidence.

## Model

The default model is:

```text
gpt-5.5
```

Override it with:

```bash
--openai-model gpt-5.5
```

or:

```bash
export BETTING_OPENAI_MODEL=gpt-5.5
```

The implementation uses the OpenAI Responses API.

## Local Run

```bash
OPENAI_API_KEY=... cargo run -- examples/norsk_tipping_candidates.csv \
  --date 2026-05-15 \
  --research examples/research_sources.txt \
  --ai \
  --openai-model gpt-5.5
```

## GitHub Actions

Add this repository secret:

```text
OPENAI_API_KEY
```

The existing workflow enables the AI path with:

```yaml
BETTING_ENABLE_AI: "true"
BETTING_OPENAI_MODEL: gpt-5.5
OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

If `OPENAI_API_KEY` is missing, the workflow fails rather than silently
publishing a non-AI report.

## Official API References

- Responses API: https://platform.openai.com/docs/api-reference/responses
- Text generation with Responses: https://platform.openai.com/docs/guides/text
- Structured outputs: https://platform.openai.com/docs/guides/structured-outputs
- Models: https://platform.openai.com/docs/models
