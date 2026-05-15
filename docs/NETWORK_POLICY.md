# Network Policy

## Principle

The VM may have three network legs, but containers must not automatically receive
all three. A container only gets the VLANs required for its job.

Default posture:

```text
default deny
explicit allow
no lateral movement
no implicit routing between VLANs
```

## VLAN Roles

`ingress`

- Receives approved inbound application traffic.
- No direct internet egress unless explicitly approved.
- No log collector access unless the service is also a log shipper.

`egress`

- Outbound-only through an allowlisted proxy, firewall, or NAT gateway.
- No inbound service exposure.
- DNS must use approved resolvers.
- Public API access must be documented per destination.

`logs`

- Outbound log shipping only.
- Prefer mTLS to the collector.
- No default route to the internet.
- No administrative access.
- No application control traffic.

## Container Attachment Rules

Allowed normal patterns:

- ingress-only service
- egress-only worker
- ingress plus logs
- egress plus logs

High-risk exception:

- ingress plus egress plus logs

The high-risk pattern requires a documented exception because a compromised
container could become a bridge across all three VLANs.

## Host Rules

- Disable IP forwarding unless the VM is explicitly a firewall/router.
- Disable source routing and redirects.
- Enforce host firewall default deny between interfaces.
- Do not expose container runtime sockets on any workload VLAN.
- Keep management access on a separate management network.
- Do not allow containers to create network devices or alter host firewall
  rules.

## Runtime Rules

Containers must not use:

- `--network host`
- `--privileged`
- `CAP_NET_ADMIN`
- `NET_RAW`
- host PID namespace
- host IPC namespace
- Docker/Podman socket mounts

Exceptions must be documented and approved before deployment.

## Logging Rules

- Logs flow out over the `logs` VLAN.
- Logs must not contain secrets, auth headers, API keys, or classified content.
- Log forwarding failures must be visible.
- Clock skew must be monitored because timestamps are evidence.
