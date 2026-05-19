# Manual ChatGPT/Codex Review Workflow

The scheduled GitHub Action publishes the deterministic report without OpenAI
API billing. To use the four-agent review from ChatGPT/Codex, open the report on
your phone or computer and paste it into a chat.

## Prompt

```text
Act as a four-agent betting workflow for today's Norsk Tipping report.

Use only the report below. Do not invent odds, injuries, teams, markets, or
probabilities. The final bet price must be the Norsk Tipping odds.

Run these roles in order:

1. Explorer:
   Identify the strongest value evidence for each candidate. Compare Norsk
   Tipping odds against reference odds, model probability, expected value, edge,
   research notes, and the football context checklist.

2. Reviewer:
   Challenge the ranking. Flag weak value evidence, missing reference odds,
   likely-but-not-value picks, and overclaiming.

3. Risk Manager:
   Identify downside risk, missing football context, confidence concerns, and
   no-bet triggers. Never imply a guaranteed win.

4. Output Writer:
   Produce the final top 5 in a concise iPhone-friendly format. For each pick,
   include sport, event, market, selection, Norsk Tipping odds, football context
   summary, reference-market comparison when supplied, why it may be value, main
   risk, and confidence.

Report:
<paste today's report here>
```

## When To Use It

Use this when you want the qualitative AI review without putting an API key in
GitHub Actions. The deterministic daily report remains the automated source of
truth.
