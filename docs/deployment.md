# Deployment Engine & Orchestration

Moonships executes Git/Docker operations on the **selected remote server** over hardened SSH. v0.3 introduced immutable revisions, rollback, safe retry/cancel, and worker execution leases. v0.4 adds a managed ingress path that starts candidates beside the active runtime and switches traffic only after health succeeds.

## Two Runtime Modes

### Managed ingress / zero-downtime mode

Activated when an application revision has at least one domain and **no published host port**.

```
queued
  -> connecting
  -> cloning
  -> building
  -> starting_candidate
  -> healthchecking
  -> switching_traffic
  -> draining_old
  -> success | failed
```

The candidate container:
- has a deterministic revision-specific runtime name;
- joins the `moonships-ingress` Docker network;
- does not publish a host port;
- is healthchecked over its Docker-network address before it can receive traffic.

After health succeeds, Moonships writes a complete Traefik file-provider route to a temporary file and atomically renames it into place. Only then is the revision promoted to current known-good and the previous runtime drained/removed.

If candidate start or healthcheck fails, the candidate is removed and the existing active runtime stays serving traffic.

### Direct published-port mode

Applications that explicitly configure a published host port retain the compatibility replacement path:

```
queued -> connecting -> cloning -> building
       -> stopping_old -> starting_new -> healthchecking
       -> success | failed
```

This path is intentionally not advertised as zero-downtime.

## Managed Traefik

Moonships can maintain a target-side Traefik container named `moonships-traefik` on network `moonships-ingress`.

Configuration:
- `MOONSHIPS_TRAEFIK_IMAGE` defaults to `traefik:v3.1`;
- `MOONSHIPS_ACME_EMAIL` is required when managed HTTPS domains exist;
- ports 80 and 443 on the target must be available to the managed proxy;
- dynamic route files live under `$HOME/.moonships/traefik/dynamic`;
- ACME state lives in a target-side `acme.json` with restrictive permissions.

Moonships starts an existing managed proxy rather than replacing an arbitrary external Traefik installation. Operators should not reuse the reserved container/network names for unrelated workloads.

## Domain Verification & TLS

Before managed traffic is switched, each revision domain must resolve to the selected target server. Domain state records:
- `verification_status`;
- `verified_at`;
- `tls_status`;
- `last_error`.

HTTPS routes use Let's Encrypt HTTP-01 through the managed Traefik resolver. HTTP requests for HTTPS-enabled domains are redirected to HTTPS. TLS readiness is observable separately from deployment success because ACME issuance may complete shortly after a healthy route switch.

## Reliability Boundary

- Deployment revisions are immutable and retain encrypted rollback material.
- Retry/rollback target the recorded source commit/configuration, not current mutable app config.
- Execution leases prevent duplicate workers from advancing the same deployment.
- Candidate runtime identity is deterministic, allowing stale worker recovery to reconcile an already-running candidate.
- Current/previous known-good revisions are protected from retention cleanup.

## Security Boundary

- Operational API routes require JWT.
- SSH uses strict host-key checking and optional fingerprint pinning.
- Secrets remain encrypted at rest and are sent to target runtimes through SSH stdin-backed temporary env files.
- The control plane does not mount a Docker socket.
- Managed route writes use an atomic temporary-file replacement rather than partial in-place mutation.

## Known Limitations

- Zero-downtime applies to managed-ingress applications, not explicit published-port deployments.
- HTTP-01 requires public reachability of ports 80/443 and correct DNS.
- Automatic DNS provider integration / DNS-01 is not part of v0.4.
