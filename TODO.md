# MOONSHIPS — Task Index

GitHub Issues are the canonical source of truth.

## Released
- [x] v0.2.0 Secure Remote Deployment
- [x] #46 selected-server remote execution
- [x] #47 JWT management API protection
- [x] #51 remote lifecycle/security hardening
- [x] #52 v0.2 release gates
- [x] #53 v0.2 release

## Grand Plan to v1
- [ ] #57 Grand Plan: Moonships v1.0 Production GA
- [x] #58 v0.3 Deployment Reliability, Revisions, Cancellation & Rollback
  - [x] #66 immutable deployment revisions
  - [x] #67 cancel and retry
  - [x] #68 one-click rollback
  - [x] #69 idempotent/restart-safe worker
  - [x] #70 retention and cleanup
- [x] #59 v0.4 Managed Ingress, TLS & Zero-Downtime
  - [x] #72 managed Traefik runtime/network
  - [x] #73 blue/green health-before-switch
  - [x] #74 domain verification + ACME TLS lifecycle
- [x] #60 v0.5 Git Provider Automation & Preview Deployments
  - [x] #75 Git provider abstraction + encrypted credentials
  - [x] #76 signed webhooks + dedupe + status callbacks
  - [x] #77 pull/merge-request preview lifecycle
- [x] #61 v0.6 Production Operations, Backup, Restore & Observability
  - [x] #78 metrics and operational health
  - [x] #79 scheduled backup + verified restore
  - [x] #80 notifications + retention visibility + runbooks
- [x] #62 v0.7 Organizations, RBAC, API Tokens & Audit Trail
  - [x] #81 organizations + memberships + route RBAC
  - [x] #82 scoped API tokens + session revocation
  - [x] #83 immutable audit + auth abuse controls
- [ ] #63 v0.8 PostgreSQL Scale Adapter, Multi-Worker & Advanced Workloads
  - [ ] #31 PostgreSQL scale adapter
- [ ] #64 v0.9 GA Hardening, Upgrade Safety & Release Candidate
- [ ] #65 v1.0 Moonships Production GA

## Superseded
- #32 broad advanced-platform umbrella is superseded by #57–#65.
