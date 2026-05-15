#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "security check failed: $*" >&2
  exit 1
}

if git ls-files | grep -E '(^|/)\.env$|(^|/)public/' >/dev/null; then
  fail "ignored runtime files are tracked"
fi

if rg -n --hidden --glob '!.git' --glob '!target' --glob '!Cargo.lock' \
  'sk-[A-Za-z0-9_-]{20,}|ghp_[A-Za-z0-9_]{20,}|-----BEGIN (RSA |EC |OPENSSH |)PRIVATE KEY-----' .; then
  fail "possible committed secret detected"
fi

if rg -n --hidden --glob '!.git' --glob '!target' \
  '(^|[[:space:]])--privileged([[:space:]]|$)|privileged:[[:space:]]*true|network_mode:[[:space:]]*host|hostNetwork:[[:space:]]*true' .; then
  fail "privileged or host-network container setting detected"
fi

if rg -n --hidden --glob '!.git' --glob '!target' \
  --glob '!docs/**' --glob '!scripts/security_static_checks.sh' \
  'CAP_NET_ADMIN|CAP_SYS_ADMIN|NET_RAW|SYS_PTRACE|/var/run/docker\.sock|/run/podman/podman\.sock' .; then
  fail "dangerous container capability or runtime socket reference detected"
fi

if rg -n --hidden --glob '!.git' --glob '!target' --glob '!docs/**' \
  ':[[:space:]]*latest($|[[:space:]])|:latest($|[[:space:]])' .; then
  fail "floating latest image tag detected outside docs"
fi

echo "security static checks passed"
