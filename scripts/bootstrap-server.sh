#!/usr/bin/env bash
# ==============================================================================
# Moonships Target Server Bootstrap Script (POSIX Bash)
# ==============================================================================
# Prepares a remote Linux server (Ubuntu/Debian/Rocky/RHEL) to host Moonships-
# deployed applications via Docker.
#
# SAFETY GUARANTEES:
# - Requires explicit interactive confirmation or --yes flag.
# - Never disables SSH or modifies existing firewall rules silently.
# - Contains no hard-coded passwords or private keys.
# - Idempotent: safe to run multiple times without destructive effects.
# ==============================================================================

set -euo pipefail

echo "======================================================================"
echo "           MOONSHIPS TARGET SERVER BOOTSTRAP SCRIPT                  "
echo "======================================================================"
echo "This script prepares the current machine for Moonships orchestration by:"
echo " 1. Verifying system architecture and OS release"
echo " 2. Installing official Docker Engine and Docker Compose plugin"
echo " 3. Adding current deploy user to the 'docker' group"
echo " 4. Ensuring Docker daemon is enabled and running"
echo ""
echo "NOTICE: This script will NOT disable your firewall or alter SSH configs."
echo "======================================================================"

AUTO_CONFIRM=false
if [ "${1:-}" = "--yes" ] || [ "${1:-}" = "-y" ]; then
    AUTO_CONFIRM=true
fi

if [ "$AUTO_CONFIRM" = false ]; then
    read -rp "Proceed with Docker installation and preflight setup? [y/N]: " confirm
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        echo "Bootstrap cancelled by operator."
        exit 0
    fi
fi

# Detect root / sudo
SUDO=""
if [ "$(id -u)" -ne 0 ]; then
    if command -v sudo >/dev/null 2>&1; then
        SUDO="sudo"
    else
        echo "Error: root privileges or sudo required to install Docker." >&2
        exit 1
    fi
fi

echo "--> Detecting operating system..."
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_ID=$ID
else
    echo "Error: Cannot identify Linux distribution via /etc/os-release." >&2
    exit 1
fi

echo "--> Target OS detected: $OS_ID ($VERSION_ID)"

if command -v docker >/dev/null 2>&1; then
    echo "--> Docker is already installed: $(docker --version)"
else
    echo "--> Installing Docker Engine..."
    case "$OS_ID" in
        ubuntu|debian)
            $SUDO apt-get update
            $SUDO apt-get install -y ca-certificates curl gnupg
            $SUDO install -m 0755 -d /etc/apt/keyrings
            if [ ! -f /etc/apt/keyrings/docker.gpg ]; then
                curl -fsSL "https://download.docker.com/linux/$OS_ID/gpg" | $SUDO gpg --dearmor -o /etc/apt/keyrings/docker.gpg
                $SUDO chmod a+r /etc/apt/keyrings/docker.gpg
            fi
            echo \
              "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/$OS_ID \
              $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
              $SUDO tee /etc/apt/sources.list.d/docker.list > /dev/null
            $SUDO apt-get update
            $SUDO apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
            ;;
        centos|rhel|rocky|almalinux|fedora)
            $SUDO yum install -y yum-utils
            $SUDO yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
            $SUDO yum install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
            ;;
        *)
            echo "Unsupported distribution: $OS_ID. Please install Docker Engine manually." >&2
            exit 1
            ;;
    esac
fi

echo "--> Ensuring Docker daemon is enabled and active..."
$SUDO systemctl enable --now docker

CURRENT_USER=$(id -un)
echo "--> Adding user '$CURRENT_USER' to docker group..."
$SUDO usermod -aG docker "$CURRENT_USER" || true

echo "--> Verifying Docker service status..."
if docker info >/dev/null 2>&1; then
    echo "--> Docker engine is responsive and accepting commands."
elif $SUDO docker info >/dev/null 2>&1; then
    echo "--> Docker engine is running with sudo. (Log out and back in for group membership to take effect)."
else
    echo "Warning: Docker daemon is active but did not respond to 'docker info'."
fi

echo "======================================================================"
echo "          SERVER BOOTSTRAP COMPLETED SUCCESSFULLY                    "
echo "======================================================================"
echo "Next step: Add this server in the Moonships Dashboard -> Servers"
echo "Then run the Preflight Check to verify SSH and Docker reachability."
echo "======================================================================"
