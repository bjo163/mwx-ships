#!/usr/bin/env bash
# ==============================================================================
# Moonships SQLite Database Restore Script (POSIX Bash)
# ==============================================================================
# Restores an active Moonships SQLite database from a specified backup file.
# Creates a pre-restore backup of the existing state and verifies integrity.
# ==============================================================================

set -euo pipefail

BACKUP_FILE="${1:-}"
TARGET_DB="${2:-data/moonships.sqlite}"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <path-to-backup.sqlite> [target-db-path]" >&2
    exit 1
fi

if [ ! -f "$BACKUP_FILE" ]; then
    echo "Error: Backup file '$BACKUP_FILE' does not exist." >&2
    exit 1
fi

echo "========================================"
echo "  MOONSHIPS SQLITE DATABASE RESTORE"
echo "========================================"

mkdir -p "$(dirname "$TARGET_DB")"

if [ -f "$TARGET_DB" ]; then
    PRE_RESTORE="${TARGET_DB}.pre-restore-$(date +%Y%m%d%H%M%S)"
    echo "Archiving current database to '$PRE_RESTORE'..."
    cp -p "$TARGET_DB" "$PRE_RESTORE"
fi

echo "Restoring from '$BACKUP_FILE' to '$TARGET_DB'..."
cp -p "$BACKUP_FILE" "$TARGET_DB"

# Remove stale WAL or SHM indices
rm -f "${TARGET_DB}-wal" "${TARGET_DB}-shm"

if command -v sqlite3 >/dev/null 2>&1; then
    echo "Running PRAGMA integrity_check..."
    INTEGRITY=$(sqlite3 "$TARGET_DB" "PRAGMA integrity_check;")
    if [ "$INTEGRITY" = "ok" ]; then
        echo "Integrity verification passed: ok"
    else
        echo "Error: Integrity check failed: $INTEGRITY" >&2
        exit 1
    fi
fi

echo "Database restore completed successfully!"
