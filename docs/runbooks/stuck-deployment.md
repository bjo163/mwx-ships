# Runbook — Stuck Deployment / Execution Lease

## Signals
- `deployments.stale_leases > 0` in `/api/operations/metrics`.
- Dashboard reports stale work.
- `stale_deployment_lease` alert is emitted.

## Recovery
1. Inspect deployment status/logs and target state.
2. If the phase is safely cancellable, cancel from the deployment UI/API and create a retry.
3. If the execution lease expired after a worker/control-plane restart, allow the persistent worker to reclaim it. The new worker receives a new execution token and restarts from the safe reconciliation phase.
4. Do not edit deployment rows or execution tokens manually.
5. For a failed/cancelled deployment, use Retry. For a bad successful revision, use Rollback to previous known-good.

## Verification
A recovered deployment must reach a terminal state and the application current revision must correspond to the healthy runtime.
