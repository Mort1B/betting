# GitHub Security Guardrails

## What GitHub Enforces

The `security-guardrails.yml` workflow checks:

- Rust formatting,
- Rust tests,
- Rust clippy warnings,
- dependency audit,
- static repository guardrails for obvious secret and container/network mistakes.

## What It Cannot Prove

GitHub cannot prove that a NATO-cleared deployment is accredited. It can only
fail fast on mistakes in the repository.

Manual review is still required for:

- VM hardening,
- hypervisor configuration,
- firewall policies,
- VLAN mappings,
- runtime manifests,
- classified-data handling,
- operational procedures.

## Required Repository Rules

Recommended GitHub branch protection for `main`:

- require status checks to pass,
- require the security guardrails workflow,
- require pull request review for changes,
- block force pushes,
- block deletions,
- require signed commits if your organization supports it.

## Secrets

Allowed GitHub Secrets for this repository:

- `BETTING_REPORT_TOKEN`
- `OPENAI_API_KEY`

Do not create broad personal access tokens unless a future workflow explicitly
requires them.
