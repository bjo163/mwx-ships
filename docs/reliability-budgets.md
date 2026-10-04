# Moonships v1 Reliability & Performance Budgets

These are release gates and operating baselines, not marketing throughput claims.

## CI reliability budgets

| Check | v1 budget / invariant |
| --- | --- |
| Deployment state-machine soak | 128 complete attempts in under 120 seconds on GitHub-hosted CI |
| Application ownership race | exactly one winner under 32 concurrent claims |
| Execution lease recovery | stale lease reclaimable; superseded token cannot advance state |
| Lifecycle failure matrix | every active lifecycle phase reaches a clean terminal failed state under injection |
| Published-image health | /api/health ready within 60 seconds |
| Restore drill | restored DB passes integrity/schema/critical-record verification |
| Upgrade matrix | v0.2 through v0.8 fixtures reach latest without core record or ciphertext loss |

## Runtime defaults

- Deployment lease: 7200 seconds unless configured.
- Lease heartbeat must renew well before expiry.
- Provider status callback is bounded and cannot block deployment completion indefinitely.
- SSH operations use bounded phase-specific timeouts.
- Backup and retention tasks are bounded to avoid uncontrolled storage growth.
- Operational metric labels must retain bounded cardinality.

## Capacity interpretation

SQLite mode is for one control-plane node. PostgreSQL mode is required for shared-state multi-worker control planes.

Server placement resource_units is a scheduling abstraction, not CPU/RAM cgroup enforcement.

## Release policy

A material regression above these budgets blocks v0.9/v1 promotion until fixed or deliberately reclassified with documented rationale.
