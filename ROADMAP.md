# MOONSHIPS — Roadmap to v3 Production

> **Deploy. Control. Own your infrastructure.**

Moonships v1.0 is the stable production baseline. The post-v1 roadmap stays intentionally focused on one product: a self-hosted Git/Docker PaaS with stronger deployment correctness, desired-state control, and recovery semantics than typical dashboard-first platforms.

Full plan: [docs/grand-plan-v3.md](docs/grand-plan-v3.md)  
Umbrella: #90

| Version | Theme | Status |
| --- | --- | --- |
| **v1.0** | Production GA baseline | **Released** |
| **v1.1** | Fast App Creation, Detection & Deployment UX | Planned |
| **v1.2** | Stateful Services, Volumes & Backup Semantics | Planned |
| **v1.3** | Workers, Jobs & Complete Workload Lifecycle | Planned |
| **v2.0** | Declarative Desired State, Plan/Apply & Promotion | Planned |
| **v2.1** | Multi-Server Scheduling, Placement & Recovery | Planned |
| **v2.2** | Operational Intelligence, Drift & Auto-Recovery | Planned |
| **v3.0** | Production Platform GA | Planned |

## Product spine

```
Source -> Detect/Build -> Deploy -> Route -> Persist -> Observe -> Recover -> Promote/Reconcile
```

Every milestone must strengthen this path.

## v1.1 — Deployment UX
Parent #91; work #98–#100.

Deterministic repository inspection, immutable pre-queue deployment plans, and one coherent create/configure/deploy flow.

## v1.2 — Stateful Services
Parent #92; work #101–#103.

Persistent volumes plus first-class PostgreSQL, MySQL/MariaDB and Redis-compatible service lifecycle with safe backup/restore semantics.

## v1.3 — Complete Workload Lifecycle
Parent #93; work #104–#105.

Web, worker and scheduled-job roles sharing immutable build/revision identity and durable job execution history.

## v2.0 — Desired State
Parent #94; work #106–#108.

Versioned `moonships.yaml`, deterministic plan/apply, environment diff and immutable artifact promotion.

## v2.1 — Multi-Server Recovery
Parent #95; work #109–#110.

Deterministic server pools/placement, drain mode and explicit target-loss recovery without Kubernetes.

## v2.2 — Drift & Recovery Intelligence
Parent #96; work #111–#112.

Declared-vs-observed drift, reconcile preview, bounded auto-recovery, SLO counters and redacted diagnostics.

## v3.0 — Production Platform GA
Parent #97; work #113–#115.

Freeze v3 API/config contracts and prove upgrades, restores, failure recovery, supply-chain evidence and published-image behavior.

## Scope guardrail through v3

Not part of this roadmap:
- Kubernetes
- service mesh
- billing/subscriptions
- marketplace/plugins
- enterprise SAML/SCIM
- serverless
- global edge scheduling
- cloud/VPS purchasing or provisioning

Real production VPS deployment is also **not** a milestone acceptance dependency. Release readiness must be reproducible through CI/local Docker, test SSH targets, fixtures, failure injection and registry-image smoke tests.

## Branch model

Only:
- `dev` — engineering/integration
- `main` — production

No new canonical feature/release/hotfix branches.
