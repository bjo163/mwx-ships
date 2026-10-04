#!/usr/bin/env python3
"""Verified SQLite -> PostgreSQL migration for Moonships.

Requires:
- Python 3 stdlib
- psql on PATH
- target PostgreSQL schema already migrated to the same Moonships version

Examples:
  python scripts/migrate_sqlite_to_postgres.py \
    --sqlite data/moonships.sqlite \
    --postgres "$DATABASE_URL" \
    --dry-run

  python scripts/migrate_sqlite_to_postgres.py \
    --sqlite data/moonships.sqlite \
    --postgres "$DATABASE_URL" \
    --confirm MIGRATE
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
from collections import defaultdict, deque
from pathlib import Path

EXCLUDED_TABLES = {
    "sqlite_sequence",
    "seaql_migrations",
    "sqlt_loco_queue",
    "sqlt_loco_queue_lock",
    "pg_loco_queue",
}
NULL_SENTINEL = "__MOONSHIPS_NULL_9f88fdb1__"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sqlite", required=True, type=Path)
    parser.add_argument("--postgres", required=True)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--truncate-target", action="store_true")
    parser.add_argument("--confirm")
    return parser.parse_args()


def qident(value: str) -> str:
    if not value or "\x00" in value or '"' in value:
        raise ValueError(f"unsafe SQL identifier: {value!r}")
    return '"' + value.replace('"', '""') + '"'


def psql(uri: str, sql: str, *, stdin: bytes | None = None) -> str:
    env = os.environ.copy()
    env.setdefault("PGCONNECT_TIMEOUT", "10")
    result = subprocess.run(
        ["psql", uri, "-X", "-v", "ON_ERROR_STOP=1", "-At", "-c", sql],
        input=stdin,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace").strip())
    return result.stdout.decode("utf-8", errors="strict")


def sqlite_tables(conn: sqlite3.Connection) -> list[str]:
    rows = conn.execute(
        "SELECT name FROM sqlite_master "
        "WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    ).fetchall()
    return [row[0] for row in rows if row[0] not in EXCLUDED_TABLES]


def columns(conn: sqlite3.Connection, table: str) -> list[str]:
    return [row[1] for row in conn.execute(f"PRAGMA table_info({qident(table)})")]


def dependency_order(conn: sqlite3.Connection, tables: list[str]) -> list[str]:
    table_set = set(tables)
    deps: dict[str, set[str]] = {table: set() for table in tables}
    reverse: dict[str, set[str]] = defaultdict(set)

    for table in tables:
        for row in conn.execute(f"PRAGMA foreign_key_list({qident(table)})"):
            parent = row[2]
            if parent in table_set and parent != table:
                deps[table].add(parent)
                reverse[parent].add(table)

    queue = deque(sorted(table for table, parents in deps.items() if not parents))
    ordered: list[str] = []
    while queue:
        table = queue.popleft()
        ordered.append(table)
        for child in sorted(reverse[table]):
            deps[child].discard(table)
            if not deps[child]:
                queue.append(child)

    remaining = sorted(set(tables) - set(ordered))
    if remaining:
        raise RuntimeError(
            "foreign-key dependency cycle detected: " + ", ".join(remaining)
        )
    return ordered


def normalize(value):
    if value is None:
        return None
    if isinstance(value, bytes):
        return {"bytes_sha256": hashlib.sha256(value).hexdigest()}
    if isinstance(value, bool):
        return "1" if value else "0"
    text = str(value)
    lowered = text.lower()
    if lowered in {"true", "t"}:
        return "1"
    if lowered in {"false", "f"}:
        return "0"
    return text


def row_checksum(rows) -> str:
    digest = hashlib.sha256()
    normalized = [
        json.dumps([normalize(value) for value in row], separators=(",", ":"), ensure_ascii=False)
        for row in rows
    ]
    for line in sorted(normalized):
        digest.update(line.encode("utf-8"))
        digest.update(b"\n")
    return digest.hexdigest()


def sqlite_snapshot(conn: sqlite3.Connection, table: str, cols: list[str]) -> tuple[int, str]:
    projection = ",".join(qident(col) for col in cols)
    rows = conn.execute(f"SELECT {projection} FROM {qident(table)}").fetchall()
    return len(rows), row_checksum(rows)


def pg_columns(uri: str, table: str) -> list[str]:
    sql = (
        "SELECT column_name FROM information_schema.columns "
        "WHERE table_schema='public' AND table_name="
        + "'" + table.replace("'", "''") + "' ORDER BY ordinal_position"
    )
    return [line for line in psql(uri, sql).splitlines() if line]


def pg_snapshot(uri: str, table: str, cols: list[str]) -> tuple[int, str]:
    projection = ",".join(qident(col) for col in cols)
    copy_sql = (
        f"COPY (SELECT {projection} FROM {qident(table)}) "
        "TO STDOUT WITH (FORMAT csv, HEADER false, NULL '" + NULL_SENTINEL + "')"
    )
    raw = psql(uri, copy_sql)
    rows = []
    reader = csv.reader(io.StringIO(raw))
    for row in reader:
        rows.append([None if value == NULL_SENTINEL else value for value in row])
    return len(rows), row_checksum(rows)


def target_table_count(uri: str, table: str) -> int:
    out = psql(uri, f"SELECT count(*) FROM {qident(table)}").strip()
    return int(out or "0")


def export_csv(conn: sqlite3.Connection, table: str, cols: list[str], path: Path) -> int:
    projection = ",".join(qident(col) for col in cols)
    rows = conn.execute(f"SELECT {projection} FROM {qident(table)}")
    count = 0
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        for row in rows:
            encoded = []
            for value in row:
                if value is None:
                    encoded.append(NULL_SENTINEL)
                else:
                    text = value.hex() if isinstance(value, bytes) else str(value)
                    if text == NULL_SENTINEL:
                        raise RuntimeError(
                            f"{table}: data collides with migration NULL sentinel"
                        )
                    encoded.append(text)
            writer.writerow(encoded)
            count += 1
    return count


def import_csv(uri: str, table: str, cols: list[str], path: Path) -> None:
    projection = ",".join(qident(col) for col in cols)
    sql = (
        f"\\copy {qident(table)} ({projection}) FROM "
        f"'{str(path).replace(chr(39), chr(39) * 2)}' "
        f"WITH (FORMAT csv, NULL '{NULL_SENTINEL}')"
    )
    psql(uri, sql)


def reset_sequence(uri: str, table: str, cols: list[str]) -> None:
    if "id" not in cols:
        return
    escaped = table.replace("'", "''")
    sql = (
        "DO $$ DECLARE seq text; max_id bigint; BEGIN "
        f"seq := pg_get_serial_sequence('public.{escaped}', 'id'); "
        "IF seq IS NOT NULL THEN "
        f"EXECUTE 'SELECT max(id) FROM {qident(table)}' INTO max_id; "
        "IF max_id IS NULL THEN "
        "PERFORM setval(seq, 1, false); "
        "ELSE PERFORM setval(seq, max_id, true); END IF; "
        "END IF; END $$;"
    )
    psql(uri, sql)


def main() -> int:
    args = parse_args()
    if not args.sqlite.is_file():
        raise RuntimeError(f"SQLite source does not exist: {args.sqlite}")

    if not args.dry_run and args.confirm != "MIGRATE":
        raise RuntimeError("write migration requires --confirm MIGRATE")

    conn = sqlite3.connect(str(args.sqlite))
    conn.execute("PRAGMA foreign_keys=ON")
    integrity = conn.execute("PRAGMA integrity_check").fetchone()[0]
    if integrity != "ok":
        raise RuntimeError(f"SQLite integrity_check failed: {integrity}")

    tables = sqlite_tables(conn)
    order = dependency_order(conn, tables)
    if not order:
        raise RuntimeError("no Moonships tables found")

    manifest = {}
    for table in order:
        source_cols = columns(conn, table)
        target_cols = pg_columns(args.postgres, table)
        if not target_cols:
            raise RuntimeError(
                f"target table {table!r} is missing; run Moonships migrations first"
            )
        if source_cols != target_cols:
            raise RuntimeError(
                f"schema mismatch for {table}: source={source_cols} target={target_cols}"
            )
        count, checksum = sqlite_snapshot(conn, table, source_cols)
        manifest[table] = {"rows": count, "sha256": checksum, "columns": source_cols}

    print(json.dumps({"mode": "dry-run" if args.dry_run else "migrate", "tables": manifest}, indent=2))
    if args.dry_run:
        return 0

    nonempty = {table: target_table_count(args.postgres, table) for table in order}
    nonempty = {table: count for table, count in nonempty.items() if count}
    if nonempty and not args.truncate_target:
        raise RuntimeError(
            "target is not empty; refusing import without --truncate-target: "
            + json.dumps(nonempty, sort_keys=True)
        )

    if args.truncate_target:
        # Reverse dependency order avoids FK surprises and RESTART IDENTITY resets sequences.
        joined = ",".join(qident(table) for table in reversed(order))
        psql(args.postgres, f"TRUNCATE TABLE {joined} RESTART IDENTITY CASCADE")

    with tempfile.TemporaryDirectory(prefix="moonships-migrate-") as temp_dir:
        temp = Path(temp_dir)
        for table in order:
            cols = manifest[table]["columns"]
            path = temp / f"{table}.csv"
            count = export_csv(conn, table, cols, path)
            if count:
                import_csv(args.postgres, table, cols, path)
            reset_sequence(args.postgres, table, cols)

    failures = []
    for table in order:
        cols = manifest[table]["columns"]
        source_count = manifest[table]["rows"]
        source_checksum = manifest[table]["sha256"]
        target_count, target_checksum = pg_snapshot(args.postgres, table, cols)
        if source_count != target_count or source_checksum != target_checksum:
            failures.append(
                {
                    "table": table,
                    "source_rows": source_count,
                    "target_rows": target_count,
                    "source_sha256": source_checksum,
                    "target_sha256": target_checksum,
                }
            )

    invalid_constraints = psql(
        args.postgres,
        "SELECT count(*) FROM pg_constraint WHERE contype='f' AND NOT convalidated",
    ).strip()

    if failures or int(invalid_constraints or "0") != 0:
        raise RuntimeError(
            "migration verification failed: "
            + json.dumps(
                {
                    "table_mismatches": failures,
                    "unvalidated_foreign_keys": int(invalid_constraints or "0"),
                },
                indent=2,
            )
        )

    print(
        json.dumps(
            {
                "verified": True,
                "tables": len(order),
                "rows": sum(item["rows"] for item in manifest.values()),
                "foreign_keys_validated": True,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"moonships migration failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
