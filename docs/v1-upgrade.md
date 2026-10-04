# Upgrading to Moonships v1.0

Moonships v1.0 is the stable production baseline. This guide covers upgrades from every supported released schema milestone beginning with v0.2.x.

## Before you upgrade

1. Record the currently running Moonships version and immutable image digest.
2. Verify `GET /api/health` is healthy.
3. Create and verify a fresh control-plane backup.
   - SQLite: use the built-in backup + verify workflow.
   - PostgreSQL: use your database/operator backup mechanism and verify a restore point.
4. Keep the previous control-plane image digest available until post-upgrade acceptance is complete.
5. For PostgreSQL multi-worker installations, drain/stop old workers and run migrations as a separate release step with `DB_AUTO_MIGRATE=false` on workers.
6. Read the release notes for every skipped minor version.

Do not run an older Moonships binary against a database that has already been migrated by v1. If control-plane rollback is required, restore the verified pre-upgrade database backup first.

## Supported direct forward paths

The GA CI suite continuously upgrades fixtures from:

| Source | Direct schema upgrade to v1 | Important capability introduced on the path |
| --- | --- | --- |
| v0.2.x | Supported | authenticated remote deployment |
| v0.3.x | Supported | immutable revisions, retry, rollback, execution leases |
| v0.4.x | Supported | managed ingress, TLS, blue/green |
| v0.5.x | Supported | signed Git webhooks, preview deployments |
| v0.6.x | Supported | backup/restore, operations observability |
| v0.7.x | Supported | organizations, RBAC, API tokens, audit |
| v0.8.x | Supported | PostgreSQL scale mode, multi-worker, Compose, registries |
| v0.9.x | Supported | GA hardening release candidate |

The upgrade fixture preserves existing application identity and encrypted secret bytes, applies every later migration, verifies current critical tables, and runs SQLite integrity checks.

## SQLite single-node upgrade

1. Stop writes to the control plane.
2. Run a verified Moonships backup.
3. Pull the v1 image by digest.
4. Start a single v1 control-plane instance and allow/run migrations.
5. Confirm `/api/health`.
6. Verify organizations/projects/applications, server credentials, deployment history, domains, and recent backups.
7. Trigger a low-risk deployment and confirm health-before-traffic/rollback behavior where applicable.
8. Keep the pre-upgrade backup until acceptance is complete.

SQLite remains the first-class production mode for one writable Moonships control-plane node.

## PostgreSQL multi-worker upgrade

1. Verify a database restore point.
2. Drain/stop every old Moonships worker/server instance.
3. Run the v1 migration once as a dedicated release step.
4. Start one v1 instance and perform health/acceptance checks.
5. Start the remaining v1 workers.
6. Verify queue depth, worker lease heartbeat, application ownership, and target health.
7. Confirm all instances use the same `DATABASE_URL`, PostgreSQL `QUEUE_URL`, `QUEUE_KIND=Postgres`, `ENCRYPTION_KEY`, and compatible JWT/session configuration.

Mixed-version workers across a schema-changing upgrade are not supported unless a future release note explicitly says otherwise.

## Upgrading from v0.2–v0.4

These versions predate organizations/RBAC and some later recovery metadata.

After the schema upgrade:
- sign in using the existing administrator account;
- create/verify an organization and use the documented legacy-resource claim flow for unowned projects/servers where required;
- verify application revision/current-runtime state before the first production deployment;
- configure managed ingress/TLS only for applications that should use the v1 managed-ingress path.

## Upgrading from v0.5–v0.6

Review Git integrations and preview configuration:
- provider credentials remain encrypted;
- webhook secrets remain write-only through safe APIs;
- configure `MOONSHIPS_PREVIEW_BASE_DOMAIN` before accepting new preview events;
- verify backup scheduling, retention, alert endpoints, and restore procedures.

## Upgrading from v0.7

Review organization membership and token scope:
- verify every organization retains an Owner;
- rotate/revoke stale API tokens;
- verify audit history and session revocation behavior;
- check that resources intended for organization isolation are claimed/owned correctly.

## Upgrading from v0.8–v0.9

No architecture migration is required solely for v1 GA. v1 promotes the v0.9 hardened architecture into the stable contract.

For PostgreSQL installations, retain the v0.8 operational boundaries: Moonships does not operate PostgreSQL HA/backups for you.

## Post-upgrade acceptance

A production upgrade is complete only after:
- control-plane health passes;
- the expected database mode is active;
- critical resources and encrypted secrets are readable;
- no stale deployment ownership/lease remains unexpectedly;
- a deployment can be queued and reaches the expected terminal state;
- rollback is available for an application with previous known-good state;
- backup/restore status is healthy;
- organization authorization behaves as expected;
- target server health/preflight is healthy.

## Rollback

Application rollback and control-plane rollback are separate.

Application rollback uses Moonships immutable deployment revisions and does not require a database downgrade.

Control-plane rollback after v1 migrations requires:
1. stop v1;
2. restore the verified pre-upgrade database backup;
3. start the prior image digest;
4. verify health and critical records;
5. investigate the failed upgrade before retrying.

Never attempt to “downgrade” by starting an older binary against a newer migrated database unless release notes explicitly declare that path supported.

## Stable v1 boundaries

The route/method compatibility contract is `docs/openapi-v1.json`. The full support and migration policy is `docs/upgrade-policy.md`. Security assumptions and residual risk are defined in `docs/threat-model-v1.md`.
