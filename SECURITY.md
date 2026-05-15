# Security Policy

This repository is public automation code. It must not contain classified
information, operational network details, secrets, credentials, stake sizes, or
production deployment inventories.

## Boundary

The betting workflow talks to public services such as GitHub, OpenAI, Reddit,
Norsk Tipping, and public betting/research pages. It belongs on public or
low-side infrastructure.

Do not deploy this workflow inside a NATO-cleared or classified enclave unless
the accrediting authority explicitly approves every external dependency and data
flow.

## Required Secrets

Only these GitHub Actions secrets are expected:

- `BETTING_REPORT_TOKEN`
- `OPENAI_API_KEY`

Secrets must be stored in GitHub Secrets or local `.env`. They must never be
committed.

## Baselines

Security rules live in:

- `docs/SECURITY_ARCHITECTURE.md`
- `docs/NETWORK_POLICY.md`
- `docs/CONTAINER_HARDENING_BASELINE.md`
- `docs/GITHUB_SECURITY_GUARDRAILS.md`

The GitHub security workflow runs static checks, Rust checks, and dependency
audit checks.
