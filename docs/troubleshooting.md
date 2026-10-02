# Troubleshooting Guide

Common issues and remediation procedures for Moonships operators.

## 1. Database Locked (`SQLITE_BUSY`)
- **Symptom**: Error indicating `database is locked (5)`.
- **Cause**: Concurrent writers or long-running transactions without WAL mode.
- **Fix**: Verify WAL mode is active:
  ```bash
  sqlite3 data/moonships.sqlite "PRAGMA journal_mode;"
  ```
  Should return `wal`. Ensure busy timeout is set to 5000ms.

## 2. Decryption Failed on Secrets or SSH Keys
- **Symptom**: `Failed to decrypt secret` or `invalid tag`.
- **Cause**: The `ENCRYPTION_KEY` changed or does not match the key used when the secret was saved.
- **Fix**: Restore the original `ENCRYPTION_KEY` from your secrets vault.

## 3. SSH Connection Refused or Timeout
- **Symptom**: `POST /api/servers/:id/preflight` returns SSH unreachable.
- **Remediation**:
  1. Confirm target server SSH port is open.
  2. Verify public key is appended to `~/.ssh/authorized_keys` for the deploy user.
  3. Test directly: `ssh -i <key> <user>@<host> -p <port>`.

## 4. Docker Engine Inaccessible Over SSH
- **Symptom**: Preflight reports Docker command failed with permission denied.
- **Remediation**: On the target host, add the user to the `docker` group:
  ```bash
  sudo usermod -aG docker $USER
  ```
  Then log out and log back in to refresh group permissions.
