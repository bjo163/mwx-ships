# Runbook — Secret and Credential Rotation

## ENCRYPTION_KEY
`ENCRYPTION_KEY` protects SSH keys, environment secrets, Git provider credentials, and optionally whole backup exports.

Changing it without re-encryption makes existing ciphertext unreadable.

### Safe rotation
1. Create and verify a current backup and preserve the old key in an approved secrets vault.
2. Stop writes/control-plane deployment activity.
3. Re-encrypt stored secret fields with an explicit migration/rotation procedure before switching keys.
4. Start Moonships with the new key and verify SSH, Git integrations, environment secrets, rollback revision secret material, and encrypted backup verification.
5. Retain the old key according to backup retention; old encrypted exports require the key that created them.

Until an automated key-rotation command exists, do not replace `ENCRYPTION_KEY` in place.

## JWT_SECRET
Rotating `JWT_SECRET` invalidates existing JWT sessions. Schedule the change and expect all users to sign in again.

## Git/SSH credentials
Update credentials through Moonships APIs/UI so ciphertext is replaced atomically; then run server preflight or a controlled deployment.
