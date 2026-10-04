# Moonships Upgrade & Compatibility Policy

This document defines the supported upgrade contract for Moonships v1.

## Supported source versions

The v1 upgrade gate continuously tests forward schema upgrades from these released schema milestones:

| Source release | Migration prefix | Direct forward upgrade to current |
| --- | ---: | --- |
| v0.2.x | 2 | Supported |
| v0.3.x | 5 | Supported |
| v0.4.x | 6 | Supported |
| v0.5.x | 11 | Supported |
| v0.6.x | 12 | Supported |
| v0.7.x | 13 | Supported |
| v0.8.x | 15 | Supported |
| v0.9.x | 15 | Supported |

The automated fixture creates each historical schema prefix, inserts core application state plus encrypted secret bytes, applies all current migrations, and verifies data preservation and database integrity.

## Before every upgrade

1. Read the changelog and release notes for every skipped minor release.
2. Verify the current control plane is healthy.
3. Create a fresh control-plane backup.
   - SQLite: use Moonships online backup/verify tooling.
   - PostgreSQL: use the operator/database-provider backup mechanism and verify that a restore point exists.
4. Record the current image digest and Moonships version.
5. Keep the previous image available until the new release passes acceptance.
6. For multi-worker PostgreSQL installs, stop or drain all old workers before applying migrations unless the release notes explicitly declare mixed-version compatibility.

Moonships does not consider an upgrade complete until the new image passes its health check and the operator has verified critical projects/applications.

## Migration policy

Database schema migration is forward-only.

- Every released migration must be additive or data-preserving for the supported upgrade matrix.
- A migration may not silently decrypt/re-encrypt stored secrets.
- Destructive data conversion requires an explicit backup/restore plan and a major-version compatibility decision.
- Multi-instance PostgreSQL deployments should run migrations as a separate release step with DB_AUTO_MIGRATE=false on worker instances.
- SQLite remains the supported single-control-plane mode.

## Downgrade policy

Running an older Moonships binary against a database already migrated by a newer release is not supported unless the target release notes explicitly state otherwise.

To roll back the control plane after a schema-changing upgrade:

1. stop the newer Moonships process;
2. restore the pre-upgrade database backup;
3. run the previous image digest;
4. verify /api/health and critical application records;
5. investigate before attempting the upgrade again.

Application deployment rollback is independent from control-plane database downgrade.

## SQLite to PostgreSQL cutover

The verified migration utility remains the only supported automated SQLite to PostgreSQL state transfer path.

Required sequence:

1. take and verify a SQLite backup;
2. stop writes to the SQLite control plane;
3. migrate an empty, fully migrated PostgreSQL destination;
4. run the migration tool in dry-run mode;
5. execute with explicit confirmation;
6. verify row counts, checksums, and foreign keys;
7. start Moonships in PostgreSQL mode;
8. retain the SQLite backup until production acceptance completes.

There is no automatic PostgreSQL to SQLite merge after PostgreSQL has accepted new writes.

## Compatibility promise

For v1.x:

- documented v1 route/method pairs are stable;
- additive response fields and additive endpoints are permitted;
- database migrations remain forward-compatible from supported v1.x releases;
- removal or repurposing of a documented route, required field, or persisted semantic requires a new major version unless correcting a security vulnerability makes preservation unsafe.

The machine-readable route contract is docs/openapi-v1.json.
