# Agent Contract

This repository uses a two-layer agent workflow:

1. Deterministic Rust agents in `src/agents/` score and rank candidates.
2. Optional OpenAI/Codex agents described in `.agents/` review the compact
   report and write the final user-facing output.

The automated GitHub Action runs the deterministic pipeline plus the optional
OpenAI API workflow when `OPENAI_API_KEY` is configured.

## Rules

- The final bet price is always the current Norsk Tipping price.
- Candidates can be from any sport or league.
- Each pick must explain value, not only likelihood.
- Do not invent odds, injuries, probabilities, or research evidence.
- If value or confidence is insufficient, output `NO BET`.
- Keep public internet automation outside any cleared enclave unless explicitly
  approved by the accrediting authority.

## Visible Agent Definitions

- `.agents/workflows/daily_betting.md`
- `.agents/roles/explorer.md`
- `.agents/roles/reviewer.md`
- `.agents/roles/risk_manager.md`
- `.agents/roles/output_writer.md`
- `.agents/roles/deterministic_rust_agents.md`
