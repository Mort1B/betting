# Container Hardening Baseline

## Runtime

Preferred runtime on Debian VM hosts:

- rootless Podman where operationally possible,
- otherwise rootful runtime with strict daemon/socket isolation.

Required runtime controls:

- no privileged containers,
- no host networking,
- no host PID or IPC namespace,
- no container runtime socket mounts,
- no writable host path mounts unless explicitly approved,
- read-only root filesystem where possible,
- non-root user inside the container,
- all Linux capabilities dropped by default,
- AppArmor profile enabled,
- seccomp profile enabled,
- resource limits set,
- restart policy documented.

## Image Rules

Images must:

- be pinned by digest for production,
- be built from approved base images,
- avoid `latest` tags,
- include only required packages,
- run as non-root,
- produce an SBOM,
- pass vulnerability scanning before deployment,
- be signed or otherwise integrity-protected before promotion.

## Secrets

Secrets must not be:

- baked into images,
- written into logs,
- committed to git,
- passed as command-line flags visible in process listings.

Use the orchestrator's secret mechanism, mounted secret files, or environment
variables from an approved secret store.

## Filesystem

Default target:

- read-only root filesystem,
- writable tmpfs only where required,
- explicit writable data directory if persistence is required,
- no write access to application binaries,
- no package manager cache in runtime images.

## Capabilities

Default:

```text
drop all
```

Only add capabilities after documenting why they are required. The following are
blocked by default:

- `CAP_SYS_ADMIN`
- `CAP_NET_ADMIN`
- `CAP_SYS_PTRACE`
- `CAP_SYS_MODULE`
- `CAP_DAC_READ_SEARCH`
- `CAP_DAC_OVERRIDE`
- `NET_RAW`

## Deployment Approval Checklist

Before a container is allowed into a hardened environment, collect:

- image digest,
- SBOM,
- vulnerability scan result,
- runtime manifest,
- network attachments,
- environment variables and secret references,
- filesystem mounts,
- capability list,
- AppArmor/seccomp profile,
- log destinations,
- rollback plan.
