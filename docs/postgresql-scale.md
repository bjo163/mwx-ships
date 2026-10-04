# PostgreSQL Scale Mode

Moonships keeps **SQLite as the default and first-class single-control-plane production mode**. PostgreSQL is the optional scale adapter for deployments that need multiple control-plane workers sharing durable state and queue ownership.

## Supported modes

| Mode | Database | Queue | Control-plane topology |
| --- | --- | --- | --- |
| Simple production | SQLite | SQLite | One Moonships control plane |
| Scale production | PostgreSQL | PostgreSQL | Multiple Moonships server/worker processes |

Do not run multiple writable Moonships control-plane instances against the same SQLite database.

## PostgreSQL configuration

Set both persistence layers to PostgreSQL:

```bash
DATABASE_URL=postgres://moonships:...@db.example.com/moonships
QUEUE_KIND=Postgres
QUEUE_URL=postgres://moonships:...@db.example.com/moonships
DB_AUTO_MIGRATE=false
```

Run schema migration as a separate release step before starting more than one application instance:

```bash
DATABASE_URL="$DATABASE_URL" moonships-cli db migrate
```

All instances must use the same `ENCRYPTION_KEY`, JWT configuration, database, and queue.

## Multi-worker correctness boundary

Moonships uses three independent database identities:

1. **Application deployment CAS** — `applications.active_deployment_id` is claimed with a conditional update. Only one deployment intent owns an application at a time.
2. **Deployment execution lease** — a worker receives a random execution token and renewable lease. A second worker cannot advance phases while that lease is live.
3. **Webhook intent dedupe** — unique delivery/source-intent keys prevent repeated provider events from creating duplicate deployment intents.

A running deployment renews its execution lease periodically as well as at phase transitions. If the process dies, renewal stops and another worker may reclaim the deployment after lease expiry. Every subsequent phase transition still checks the execution token, so the old worker cannot advance state after ownership changes.

## SQLite to PostgreSQL migration

The supported migration utility is:

```bash
python3 scripts/migrate_sqlite_to_postgres.py \
  --sqlite data/moonships.sqlite \
  --postgres "$DATABASE_URL" \
  --dry-run
```

The dry run checks:
- SQLite integrity;
- source and target table/column parity;
- foreign-key dependency order;
- source row counts and normalized SHA-256 checksums.

After reviewing the manifest, migrate into an **empty, already-migrated target schema**:

```bash
python3 scripts/migrate_sqlite_to_postgres.py \
  --sqlite data/moonships.sqlite \
  --postgres "$DATABASE_URL" \
  --confirm MIGRATE
```

The command refuses a non-empty target unless `--truncate-target` is explicitly supplied. After import it compares row counts and normalized checksums for every migrated table and verifies that PostgreSQL has no unvalidated foreign keys.

CI performs this migration drill against a real PostgreSQL service on every `dev` and `main` change.

## Cutover procedure

1. Stop Moonships server/workers so the SQLite source is quiescent.
2. Create and migrate the PostgreSQL target schema using the **same Moonships version**.
3. Run the migration utility in `--dry-run` mode.
4. Take/retain the final SQLite backup.
5. Run the confirmed migration.
6. Start one Moonships instance with PostgreSQL and verify health, organization/resource counts, recent deployments, and secret-backed operations.
7. Start additional workers only after the first instance is verified.
8. Retain the SQLite file and backup as the rollback artifact until the PostgreSQL deployment has passed the operator acceptance window.

## Rollback

The migration does not mutate the SQLite source. Before accepting new writes on PostgreSQL, rollback is simply:

1. stop PostgreSQL-backed Moonships instances;
2. restore the pre-cutover configuration;
3. start the original SQLite control plane.

After new writes have been accepted on PostgreSQL, **do not switch back to the old SQLite file without reconciling data**. There is intentionally no automatic PostgreSQL-to-SQLite merge path in v0.8.

## Operational notes

- PostgreSQL backups are the operator/database-provider responsibility in scale mode; Moonships v0.6's SQLite online-backup job applies only to SQLite.
- Use external PostgreSQL HA/backup practices appropriate to your environment.
- Registry credentials, Git credentials, SSH keys, and application secrets remain Moonships ciphertext and require the same `ENCRYPTION_KEY` after migration.
- Queue and DB connection pools should be sized for the number of server/worker processes rather than copied blindly from single-node defaults.
