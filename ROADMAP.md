# MOONSHIPS — Roadmap to v1.0 Production GA

> **Deploy. Control. Own your infrastructure.**

The roadmap is risk-ordered. Each version removes a production failure class before the next layer is added.

| Version | Theme | Production question it answers |
| --- | --- | --- |
| **v0.2** | Secure Remote Deployment | Can Moonships deploy to the selected server securely? **Released.** |
| **v0.3** | Deployment Reliability & Rollback | Can a bad/interrupted deployment be recovered safely? |
| **v0.4** | Managed Ingress, TLS & Zero-Downtime | Can traffic move to a healthy candidate without intentional outage? **Released.** |
| **v0.5** | Git Automation & Previews | Can source-control events trigger authenticated, idempotent deployments? **Released.** |
| **v0.6** | Operations & Observability | Can operators detect, diagnose, back up, and restore production? **Released.** |
| **v0.7** | Teams, RBAC & Audit | Can multiple people operate Moonships with least privilege and accountability? **Release-ready.** |
| **v0.8** | Scale & Advanced Workloads | Can the control plane scale and support multi-container workloads without abandoning SQLite-first? |
| **v0.9** | GA Hardening / RC | Can upgrades, failures, restores, load, and security review survive release-candidate testing? |
| **v1.0** | Production GA | Is the behavior/API/upgrade contract stable enough to support as production software? |

## v0.3 — Deployment Reliability & Rollback

Parent: #58. Atomic work: #66–#70.

Deliver immutable revisions, current/previous known-good pointers, cancellation, retry, rollback, idempotent worker execution, restart recovery, and bounded cleanup/retention.

**Gate:** duplicate/restarted jobs cannot produce duplicate runtime state; previous healthy revision can be restored and healthchecked.

## v0.4 — Managed Ingress, TLS & Zero-Downtime

Parent: #59.

Manage target-side Traefik, blue/green slots, health-before-switch, atomic routing changes, automatic failed-candidate rollback, domain verification, and ACME lifecycle.

**Gate:** a failed candidate never replaces healthy traffic; a healthy candidate switches without intentional stop-old downtime.

## v0.5 — Git Provider Automation & Preview Deployments

Parent: #60.

Provider abstraction for GitHub/GitLab/Gitea, signed webhook verification, delivery deduplication, auto-deploy rules, provider status reporting, and preview lifecycle.

**Gate:** replayed/forged webhooks are rejected and duplicate deliveries remain idempotent.

## v0.6 — Production Operations, Backup & Observability

Parent: #61.

Metrics, queue/target health, operational timeline, scheduled SQLite backups, restore drills, retention, disk-pressure warnings, notifications, and disaster runbooks.

**Gate:** automated backup restores successfully into a clean instance and retention prevents unbounded growth.

## v0.7 — Organizations, RBAC, API Tokens & Audit

Parent: #62.

Organizations/workspaces, Owner/Admin/Deployer/Viewer roles, authorization boundaries, immutable audit events, scoped API tokens, revocation, and abuse controls.

**Gate:** every operational route has authorization-matrix tests and cross-organization access fails closed.

## v0.8 — Scale Adapter & Advanced Workloads

Parent: #63; PostgreSQL adapter: #31.

Keep SQLite as the supported single-node mode while adding PostgreSQL multi-worker mode, DB migration tooling, Compose/multi-container applications, private registries, and server placement.

**Gate:** the same domain/API suite passes on SQLite and PostgreSQL; multi-worker races are tested.

## v0.9 — GA Hardening / Release Candidate

Parent: #64.

Upgrade matrix, migration safety, load/soak/failure injection, restore drills, threat-model review, API contract freeze, SBOM/provenance, dependency/license audit, support matrix, and RC bug burn-down.

**Gate:** zero open P0/P1 correctness/security issues and the RC survives upgrade/restore/soak tests.

## v1.0 — Production GA

Parent: #65; umbrella: #57.

v1.0 is a stability contract rather than a feature dump. Publish only after all previous exit gates are satisfied, release artifacts are verified, published image smoke tests pass, and upgrade notes from v0.2.x are complete.
