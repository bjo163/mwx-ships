# Target Server Requirements

Moonships controls remote Linux servers over standard SSH connections. Any physical server, cloud instance, or VPS that can run Docker is supported.

## Minimum Hardware Requirements

| Resource | Minimum | Recommended |
|---|---|---|
| CPU | 1 vCPU (x86_64 or aarch64) | 2+ vCPUs |
| RAM | 1 GB | 2+ GB |
| Disk | 10 GB SSD | 30+ GB SSD |
| Network | Public IPv4 or WireGuard/Tailscale VPN | Static IPv4 / IPv6 |

## Supported Linux Operating Systems

- Ubuntu 22.04 LTS / 24.04 LTS
- Debian 11 / 12
- Rocky Linux / AlmaLinux / RHEL 9
- Fedora Server 39+

## Required Services on Target Host

1. **OpenSSH Server (`sshd`)**: Accessible from the Moonships control plane IP.
2. **Docker Engine**: Docker Community Edition 24.0+ and `docker-compose-plugin`.
3. **Deploy User**: A Linux user account with permission to execute Docker commands (typically member of the `docker` group) and an authorized SSH public key matching the server's private key registered in Moonships.
4. **Ports**:
   - `22/tcp`: SSH access (or custom SSH port).
   - `80/tcp`: HTTP ingress (handled by Traefik).
   - `443/tcp`: HTTPS ingress (handled by Traefik).
