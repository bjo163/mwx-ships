# MOONSHIPS — Roadmap to v1.0 Production GA

> **Deploy. Control. Own your infrastructure.**

Moonships follows a risk-ordered release train. A capability is not marked released until its exact production commit passes CI, security, Docker smoke, published-image smoke, tagging, and GitHub Release creation.

| Version | Theme | Status |
| --- | --- | --- |
| **v0.2** | Secure Remote Deployment | **Released** |
| **v0.3** | Deployment Reliability & Rollback | **Released** |
| **v0.4** | Managed Ingress, TLS & Zero-Downtime | **Released** |
| **v0.5** | Git Automation & Preview Deployments | **Released** |
| **v0.6** | Operations, Backup & Observability | **Released** |
| **v0.7** | Organizations, RBAC, API Tokens & Audit | **Released** |
| **v0.8** | PostgreSQL Scale Adapter & Advanced Workloads | **Released** |
| **v0.9** | GA Hardening / RC | **In hardening** |
| **v1.0** | Production GA | Planned |

## v0.8 — Scale Adapter & Advanced Workloads

Parent #63; work #31, #84–#86.

SQLite remains the first-class simple production mode. PostgreSQL adds shared state/queue operation for multiple control-plane workers. The release gate requires the same application suite on both databases, a verified SQLite→PostgreSQL migration drill, atomic deployment ownership under concurrency, execution lease heartbeat/recovery, private registry safety, Compose lifecycle/log aggregation, deterministic server placement, security audit, and production container smoke.

## v0.9 — GA Hardening / Release Candidate

Parent #64; work #87–#89.

Freeze supported upgrade paths and v1 API behavior. Add migration fixtures from supported older versions, load/soak/failure injection, restore and target-loss drills, performance/resource budgets, threat-model review, dependency/license gates, SBOM/provenance, and published-artifact verification. v0.9 is a release-candidate phase, not a feature sprint.

## v1.0 — Production GA

Parent #65; umbrella #57.

v1.0 is a stability contract: documented upgrade/support policy, tested disaster recovery, least-privilege authorization, dual database support boundaries, reproducible release metadata, supply-chain artifacts, zero unresolved P0/P1 correctness/security issues, and a published image verified after registry pull.
