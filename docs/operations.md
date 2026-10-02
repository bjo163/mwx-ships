# Moonships Operations Runbook

This guide covers operational best practices for maintaining a healthy Moonships control plane.

## Process Supervision

In production, run Moonships as a systemd service or inside the official Docker container.

### Systemd Service Unit (`/etc/systemd/system/moonships.service`)
```ini
[Unit]
Description=Moonships Control Plane
After=network.target

[Service]
Type=simple
User=moonships
WorkingDirectory=/opt/moonships
Environment="PORT=5150"
Environment="ENCRYPTION_KEY=your_32_byte_hex_encryption_key"
Environment="DATABASE_URL=sqlite:///opt/moonships/data/moonships.sqlite?mode=rwc"
ExecStart=/opt/moonships/moonships-cli start --server-and-worker
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## Storage & Volume Management

- Ensure `data/` resides on persistent non-ephemeral storage.
- If using Docker, map `/data` to a named volume:
  ```yaml
  volumes:
    - moonships_data:/data
  ```
- Regularly run `./scripts/backup-sqlite.sh` as a cron job.

## Log Maintenance

Deployment logs are stored inside the `deployment_logs` table. To inspect database size:
```bash
ls -lh data/moonships.sqlite
```
To purge old deployment logs beyond 90 days, you can run a scheduled SQL query:
```sql
DELETE FROM deployment_logs WHERE created_at < datetime('now', '-90 days');
VACUUM;
```
