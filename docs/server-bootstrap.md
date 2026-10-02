# Target Server Bootstrap Guide

To prepare a fresh Linux host for Moonships management, you can use our safe bootstrap script: `scripts/bootstrap-server.sh`.

## Bootstrap Philosophy & Safety Principles

1. **Explicit Confirmation**: The script requires interactive confirmation (`y/N`) unless explicitly invoked with `--yes`.
2. **No Silent Firewall Alterations**: The script will NOT modify or disable your UFW, firewalld, or iptables rules.
3. **No SSH Tampering**: The script will NOT alter your `sshd_config` or disable password/root logins unexpectedly.
4. **No Plaintext Passwords**: No credentials or private keys are written to the machine.
5. **Idempotence**: Safe to run on already-configured servers; missing packages will be installed, while existing components remain intact.

## Quick Bootstrap Command

On the target Linux machine:

```bash
# Download and review the bootstrap script
curl -fsSL https://raw.githubusercontent.com/bjo163/mwx-ships/master/scripts/bootstrap-server.sh -o bootstrap-server.sh
chmod +x bootstrap-server.sh

# Run interactively
sudo ./bootstrap-server.sh
```

## What the Script Does

1. Detects OS distribution (Ubuntu, Debian, RHEL, Rocky, AlmaLinux).
2. Installs official Docker repository GPG keys and package lists.
3. Installs `docker-ce`, `docker-ce-cli`, `containerd.io`, and `docker-compose-plugin`.
4. Enables and starts the `docker` systemd service.
5. Adds the current operator user to the `docker` group so rootless Docker execution is permitted over SSH.
6. Verifies engine health via `docker info`.
