#!/usr/bin/env bash
# ==============================================================================
# Moonships SQLite Online Backup Script (POSIX Bash)
# ==============================================================================
# Performs an atomic, zero-downtime online backup of the Moonships SQLite database
# using SQLite .backup or VACUUM INTO to safely capture WAL journal state.
# ==============================================================================

set -euo pipefail

SOURCE_DB="${2:-data/moonships.sqlite}"
DESTINATION="${1:-}"

echo "========================================"
echo "  MOONSHIPS SQLITE ONLINE BACKUP"
echo "========================================"

if [ ! -f "$SOURCE_DB" ]; then
    echo "Error: Source database '$SOURCE_DB' does not exist." >&2
    exit 1
fi

if [ -z "$DESTINATION" ]; then
    mkdir -p backups
    TIMESTAMP=$(date +%Y%m%d-%H%M%S)
    DESTINATION="backups/moonships-${TIMESTAMP}.sqlite"
else
    mkdir -p "$(dirname "$DESTINATION")"
fi

echo "Source DB   : $SOURCE_DB"
echo "Destination : $DESTINATION"

if command -v sqlite3 >/dev/null 2>&1; then
    echo "Executing atomic online backup via sqlite3 CLI..."
    sqlite3 "$SOURCE_DB" ".backup '$DESTINATION'"
else
    echo "Warning: sqlite3 CLI not found. Copying main and WAL files..."
    cp -p "$SOURCE_DB" "$DESTINATION"
    if [ -f "${SOURCE_DB}-wal" ]; then
        cp -p "${SOURCE_DB}-wal" "${DESTINATION}-wal"
    fi
fi

if [ -f "$DESTINATION" ]; then
    SIZE=$(stat -c%s "$DESTINATION" 2>/dev/null || stat -f%z "$DESTINATION" 2>/dev/null || wc -c < "$DESTINATION")
    echo "Backup completed successfully! Size: $SIZE bytes"
    echo "CRITICAL: Ensure your ENCRYPTION_KEY environment variable is also securely recorded."
else
    echo "Error: Backup destination was not created." >&2
    exit 1
fi
