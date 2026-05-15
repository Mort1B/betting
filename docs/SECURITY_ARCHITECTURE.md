# Security Architecture

## Goal

Build the project so it can be reviewed and adapted for hardened Debian VM and
container deployments without mixing public internet automation into a cleared
environment.

## Non-Negotiable Boundary

This betting workflow consumes public services:

- GitHub Actions and GitHub Pages
- OpenAI API
- Reddit and public web pages
- Norsk Tipping and betting reference data

That means this workflow is not suitable for direct deployment inside a
NATO-cleared or classified enclave unless the accreditation authority approves
the complete external data path. Keep it outside the cleared network by default.

## Deployment Zones

`public automation zone`

- Runs this repository's GitHub Actions workflow.
- Publishes the daily report to GitHub Pages under a long random token path.
- May call OpenAI API and public research sources.
- Contains no classified inputs, outputs, logs, or credentials beyond GitHub
  Secrets.

`controlled operations zone`

- Can read the report manually if approved.
- Must not provide direct network reachability back into the cleared network.
- Must not copy classified data into the public workflow.

`cleared enclave`

- No direct dependency on this repository's public internet workflow.
- Any import must go through an approved cross-domain or manual review process.
- No OpenAI, GitHub, Reddit, Norsk Tipping, or public betting page traffic unless
  explicitly authorized.

## VM Baseline

All Debian VMs used for container hosting should have:

- hardened golden images,
- full patch and vulnerability evidence,
- AppArmor enabled,
- audit logging enabled,
- host firewall default deny,
- no unnecessary services,
- no interactive admin access from workload VLANs,
- time synchronization from approved sources,
- centralized log forwarding,
- immutable or tightly controlled system configuration.

## Hypervisor Boundary

HPE VM Essentials/KVM is part of the accreditation boundary. Harden and document:

- management plane access,
- administrator roles,
- VM template provenance,
- virtual switch/VLAN mappings,
- snapshot/backup controls,
- storage isolation,
- migration network isolation,
- patch cadence,
- audit logging.

## Evidence Required Before Production

- VM hardening checklist result.
- Container runtime hardening checklist result.
- Image digest and SBOM for every deployed container.
- Vulnerability scan result for every image.
- Network policy/firewall review.
- Log redaction review.
- Secrets handling review.
- Approval for every external data flow.
