# SQLite Backup and Disaster Recovery Runbook

Because Moonships stores both control plane state and the persistent deployment queue inside SQLite, backups are straightforward, atomic, and zero-downtime.

## Online Backup Mechanisms

Never copy an active SQLite database with simple file copy (`cp`) while WAL transactions are in flight, as you may copy inconsistent journal pages.

Use the provided backup scripts which utilize the SQLite Online Backup API:

### Linux / macOS (Bash)
```bash
# Run online backup to default location (./backups/moonships-YYYYMMDD-HHMMSS.sqlite)
./scripts/backup-sqlite.sh

# Or specify a custom target path
./scripts/backup-sqlite.sh /var/backups/moonships-snapshot.sqlite
```

### Windows (PowerShell)
```powershell
# Run online backup
.\scripts\backup-sqlite.ps1

# Custom path
.\scripts\backup-sqlite.ps1 -Destination "D:\Backups\moonships.sqlite"
```

## Critical Prerequisite: Encryption Key Protection

Moonships encrypts SSH private keys and sensitive application secrets with AES-256-GCM using `ENCRYPTION_KEY`.
> **WARNING:** If you lose your `ENCRYPTION_KEY`, restored databases CANNOT decrypt SSH keys or environment secrets! Always store your `ENCRYPTION_KEY` securely in a password manager or secrets vault alongside your database backups.

## Disaster Recovery & Restore Procedure

To restore an active Moonships instance from a backup file:

1. Stop the Moonships service or container to release file handles.
2. Execute the restore script:

### Linux / macOS
```bash
./scripts/restore-sqlite.sh /var/backups/moonships-snapshot.sqlite data/moonships.sqlite
```

### Windows
```powershell
.\scripts\restore-sqlite.ps1 -BackupFile "D:\Backups\moonships.sqlite" -TargetDb "data/moonships.sqlite"
```

3. The script automatically creates a pre-restore archive of the previous database and runs `PRAGMA integrity_check;` to verify database validity before completion.
4. Restart Moonships:
```bash
cargo loco start --server-and-worker
```
