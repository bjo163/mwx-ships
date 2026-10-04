# Target Server Requirements

Moonships controls remote Linux servers over OpenSSH. Any reachable Linux host that can run Docker and the required command-line utilities can be a deployment target.

## Minimum Hardware Requirements

| Resource | Minimum | Recommended |
|---|---|---|
| CPU | 1 vCPU | 2+ vCPUs |
| RAM | 1 GB | 2+ GB |
| Disk | 10 GB SSD | 30+ GB SSD |
| Network | Reachable SSH endpoint | Static address or private VPN |

## Tested/Expected Linux Families

- Ubuntu 22.04 / 24.04
- Debian 11 / 12
- Rocky Linux / AlmaLinux / RHEL 9
- Comparable modern Linux distributions with OpenSSH and Docker

## Required Target Software

1. **OpenSSH Server (`sshd`)** reachable from the Moonships control plane.
2. **Docker Engine 24+** with permission for the configured deploy user to run `docker info`, build/pull images, and manage containers.
3. **Git** for remote repository clone/fetch/checkout.
4. **curl** for target-local HTTP healthchecks.
5. A deploy user with an authorized SSH public key matching the encrypted private key stored in Moonships.

## SSH Host Verification

For production, obtain the target's SHA256 SSH host fingerprint through a trusted out-of-band channel and store it in the server's `known_host_fingerprint` field. Moonships refuses the connection when a configured fingerprint does not match.

## Network Ports

- SSH port (22/tcp by default) must be reachable from the control plane.
- For Moonships-managed ingress, TCP 80 and 443 must be available on the target for the managed Traefik container.
- Published application ports used for healthchecks must be reachable from the target host itself.


## Managed ingress target requirements

For v0.4 managed ingress:
- the deploy user must be able to create Docker networks and containers;
- ports 80/tcp and 443/tcp must not already be owned by an unrelated service;
- public HTTPS domains must resolve to the target and permit Let's Encrypt HTTP-01 traffic;
- application healthchecks are required before Moonships can switch managed traffic.
