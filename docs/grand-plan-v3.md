# Moonships v3 Production Grand Plan

Umbrella issue: #90.

Moonships v1.0 established the secure/recoverable production baseline. The v3 roadmap is deliberately narrower than a generic cloud platform: become an excellent self-hosted PaaS for Git/Docker workloads first, then add deterministic desired-state and multi-server recovery.

## Product spine

```
Source
  -> Detect/Build
  -> Deploy
  -> Route
  -> Persist
  -> Observe
  -> Recover
  -> Promote/Reconcile
```

Every roadmap item must strengthen this spine. If a proposed capability does not materially improve it, it waits until after v3.

## Scope guardrails through v3

Explicitly out of scope:
- Kubernetes orchestration
- service mesh
- billing/subscription systems
- plugin/marketplace ecosystem
- enterprise SAML/SCIM
- global edge scheduling
- serverless/functions platform
- cloud/VPS purchasing or provider provisioning
- requiring a real production VPS as a release gate

Acceptance remains reproducible with unit/integration tests, local/CI Docker, mock or containerized SSH targets, database fixtures, failure injection, and published-image smoke tests.

## Release train

### v1.1 — Fast App Creation, Detection & Deployment UX
Parent #91; work #98–#100.

Reach the expected modern self-hosted PaaS onboarding experience without introducing hidden magic. Repository inspection is non-executing, all inference has reason/confidence metadata, manual override stays available, and an immutable deployment plan exists before queueing.

### v1.2 — Stateful Services, Volumes & Backup Semantics
Parent #92; work #101–#103.

Make PostgreSQL, MySQL/MariaDB, Redis-compatible services and persistent volumes first-class. Moonships owns lifecycle metadata, secret safety and backup/restore semantics, but does not pretend to be a full HA database operator.

### v1.3 — Workers, Jobs & Complete Workload Lifecycle
Parent #93; work #104–#105.

Complete ordinary application primitives: web, worker and scheduled jobs sharing one immutable artifact/revision identity, with durable execution history, lease/deduplication semantics and consistent logs.

### v2.0 — Declarative Desired State, Plan/Apply & Promotion
Parent #94; work #106–#108.

Introduce `moonships.yaml`, deterministic plan/apply, environment diff and immutable artifact promotion. This is the main differentiation from dashboard-only PaaS workflows: changes become inspectable state transitions instead of opaque button presses.

### v2.1 — Multi-Server Scheduling, Placement & Recovery
Parent #95; work #109–#110.

Extend the existing SSH/Docker execution model with deterministic server pools, placement scoring, drain/maintenance and explicit target-loss recovery. Do this without Kubernetes and without hiding stateful-storage constraints.

### v2.2 — Operational Intelligence, Drift & Auto-Recovery
Parent #96; work #111–#112.

Compare declared intent with observed runtime state, produce deterministic reconcile previews, and add tightly bounded auto-recovery. Observation is read-only by default; recovery is auditable, rate-limited and cannot loop indefinitely.

### v3.0 — Production Platform GA
Parent #97; work #113–#115.

Freeze the focused platform into a stable contract: API/config compatibility, supported v1/v2 upgrade paths, restore/failure drills, published-image validation and supply-chain evidence.

## Why this should beat broader platforms

Moonships should compete on correctness and operator clarity, not feature count:

1. deployment plan exists before execution;
2. source/config/artifact identity is immutable and traceable;
3. promotion reuses known-good artifacts instead of rebuilding;
4. drift is explicit instead of silently tolerated;
5. reconcile is previewable before mutation;
6. placement decisions have deterministic reason codes;
7. recovery is bounded and observable;
8. the control plane keeps the no-Docker-socket security boundary;
9. SQLite stays simple for one node while PostgreSQL provides the scale path.

## Branch invariant

Only two canonical branches:
- `dev` — engineering/integration;
- `main` — production.

No canonical feature, release, dependency or hotfix branches.

## Production-readiness interpretation

“Production” through this roadmap means the repository and published artifacts pass reproducible production-grade release gates. Deployment onto a real operator VPS is a separate rollout step and is intentionally not required for these milestones.
