# OpenAI API Agent Workflow

The daily GitHub Action uses the OpenAI API when `OPENAI_API_KEY` is configured
as a repository secret. This is pay-as-you-go API usage and is separate from any
ChatGPT subscription.

The deterministic Rust report runs through four OpenAI-backed agents after the
filters have ranked the candidates.

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
  reference-market comparison, rationale, risks, strict-rule status, and
  confidence score out of 100.
- Preserves fallback warnings when the deterministic report had to fill the top
  3 from best available candidates.

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
OPENAI_API_KEY=... cargo run -- --norsk-tipping-live \
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

The scheduled workflow enables the API path:

```yaml
BETTING_ENABLE_AI: "true"
BETTING_OPENAI_MODEL: gpt-5.5
OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
```

If your account does not have access to `gpt-5.5`, change
`BETTING_OPENAI_MODEL` in `.github/workflows/daily-report.yml` to the exact model
available in your OpenAI project.

## Official API References

- Responses API: https://platform.openai.com/docs/api-reference/responses
- Text generation with Responses: https://platform.openai.com/docs/guides/text
- Structured outputs: https://platform.openai.com/docs/guides/structured-outputs
- Models: https://platform.openai.com/docs/models
