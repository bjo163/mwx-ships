# Server Preflight Inspection

Before deploying workloads to a registered server, Moonships executes an automated preflight check via:
`POST /api/servers/:id/preflight`

## Preflight Verification Checks

1. **SSH Reachability**: Tests network round-trip and authentication using the configured SSH private key.
2. **OS & Kernel**: Reports operating system name, release, architecture, and kernel version.
3. **Docker Engine**: Confirms Docker daemon is running and responds to `docker info`.
4. **Memory Status**: Reads total memory and available memory in MB (warns if available < 256MB).
5. **Disk Space**: Inspects target partition disk usage (fails if disk usage > 90%).
6. **Workspace Write Access**: Confirms permissions to create `/var/lib/moonships` or deployment directories.

## Non-Destructive Policy

The preflight check is **strictly read-only**. It never automatically modifies, reformats, reboots, or reconfigures the target machine. If a check fails, Moonships returns a structured diagnostic report with actionable remediation guidance.
