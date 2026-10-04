# Moonships v1.0 Grand Plan

Umbrella issue: #57.

Moonships v0.2 proved the secure remote execution boundary. The path to v1 is ordered by **production risk**, not by feature visibility.

## Design principle

A PaaS is production-ready only when failure is a first-class path. The critical sequence is therefore:

```
remote execution
  -> recoverable revisions
  -> health-before-traffic
  -> event-driven automation
  -> observable/restoreable operations
  -> least-privilege multi-user control
  -> scale modes
  -> upgrade/security/load hardening
  -> stable v1 contract
```

Adding webhooks or teams before deployment rollback would amplify failure faster. Adding PostgreSQL before concurrency semantics are deterministic would move bugs to a faster database. v1 sequencing deliberately avoids those traps.

## Release train

### v0.3 — Deployment Reliability & Rollback
Issues #58, #66–#70.

Introduce immutable deployment revisions, cancellation/retry, rollback, idempotent execution/recovery, and retention. This establishes the state model needed by every later release.

### v0.4 — Managed Ingress, TLS & Zero-Downtime
Issue #59.

Add managed Traefik, blue/green runtime slots, health-before-switch, automatic route rollback, domain verification, and ACME. Traffic routing becomes a state transition instead of a container side effect.

### v0.5 — Git Provider Automation & Previews
Issue #60.

Add signed provider webhooks, idempotent delivery handling, commit status callbacks, branch rules, and preview environments. Only after the deployment state machine is safe should external events trigger it automatically.

### v0.6 — Operations, Backup & Observability
Issue #61.

Add metrics, health/queue visibility, backup scheduling, restore verification, retention, disk pressure warnings, notifications, and incident runbooks. A production control plane must be recoverable independently of application workloads.

### v0.7 — Teams, RBAC & Audit
Issue #62.

Add organizations, role-based authorization, scoped API tokens, session/token revocation, audit events, and abuse controls. Authentication becomes a true authorization model.

### v0.8 — Scale Adapter & Advanced Workloads
Issue #63; PostgreSQL work continues in #31.

Keep SQLite as the simple single-node production mode while adding optional PostgreSQL multi-worker operation, migration tooling, private registries, Compose/multi-container workloads, and placement constraints.

### v0.9 — GA Hardening / RC
Issue #64.

Freeze the v1 API and support matrix; prove upgrades, restores, failure recovery, load/soak behavior, security posture, and supply-chain/release artifacts. v0.9 is a release-candidate phase, not a feature sprint.

### v1.0 — Production GA
Issue #65.

Ship only after all previous exit gates pass. v1.0 means a stable behavior, upgrade, security, and operational contract.

## Non-goals before v1

Avoid broadening into Kubernetes orchestration, service mesh, billing, marketplace/plugins, global edge scheduling, or enterprise SSO before the core PaaS reliability contract is complete. Those can become v1.x/v2 work.

## Architecture invariants through v1

- SQLite stays first-class for single-node operation.
- Remote servers remain the workload execution boundary.
- The control plane never regains a host Docker socket dependency.
- Secrets never become plaintext persistence/log data.
- Deployment history is immutable; current state points to history rather than rewriting it.
- Traffic switches only after candidate health is proven.
- Every destructive action is authenticated, authorized, auditable, and recoverable where technically possible.
- Releases are built from `main` and validated from the published artifact, not only from source.
