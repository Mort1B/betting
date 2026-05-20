# OpenAI API Setup

The scheduled four-agent workflow uses paid OpenAI API access. This is separate
from a ChatGPT subscription.

## Steps

1. Create or open your OpenAI platform account:

```text
https://platform.openai.com
```

2. Add billing/payment in the OpenAI platform billing settings.

3. Create an API key:

```text
https://platform.openai.com/api-keys
```

4. Add it to GitHub:

```text
Repository -> Settings -> Secrets and variables -> Actions -> New repository secret
Name: OPENAI_API_KEY
Value: <your OpenAI API key>
```

5. Keep the existing report token secret:

```text
BETTING_REPORT_TOKEN
```

6. Run:

```text
Actions -> Daily Betting Report -> Run workflow
```

## Model

The workflow currently uses:

```text
gpt-5.5
```

If the workflow fails with a model access error, change
`BETTING_OPENAI_MODEL` in `.github/workflows/daily-report.yml` to a model your
OpenAI project can access.

## Cost Controls

The workflow is optimized to reduce API cost:

- deterministic Rust filtering happens before the model calls,
- only the compact top-5 report is passed to the agents,
- there are four narrow role calls,
- each call has `max_output_tokens` capped by the Rust default,
- the scheduled publisher can override that cap with
  `BETTING_AI_MAX_OUTPUT_TOKENS`.

The default cap is large enough for the Output Writer to include all five
ranked candidates. If the OpenAI response is incomplete or misses ranked
candidate headings, the publisher uses the deterministic report instead of a
partial AI report.

## References

- API authentication and API-key handling:
  https://platform.openai.com/docs/api-reference/authentication
- Developer quickstart:
  https://platform.openai.com/docs/quickstart/using-the-api
- Responses API:
  https://platform.openai.com/docs/api-reference/responses
- Text generation:
  https://platform.openai.com/docs/guides/text
