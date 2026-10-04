# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| 0.2.x | :white_check_mark: |
| 0.1.x | :white_check_mark: |

## Reporting a Vulnerability

We take the security of Moonships very seriously. If you discover a vulnerability, report it via private coordinated disclosure and do not disclose it in public issues or discussions.

Use GitHub Security Advisories to provide the vulnerability category, reproduction steps, potential impact, and any suggested mitigation.

## v0.2 Security Architecture

- **Authenticated control plane**: operational server, project, application, deployment, environment, domain, and container lifecycle routes require JWT. `GET /api/health` remains public.
- **Zero plaintext secrets at rest**: SSH private keys and secret environment variables are encrypted with AES-256-GCM using an operator-provided `ENCRYPTION_KEY`.
- **Remote execution boundary**: application Git/Docker/healthcheck work executes on the selected target server over SSH; the production control plane does not mount `/var/run/docker.sock`.
- **SSH host verification**: sessions use strict host-key checking with an isolated scanned `known_hosts` file. Operators can pin a trusted SHA256 fingerprint; without pinning, first discovery is trust-on-first-use.
- **Temporary credentials**: decrypted SSH keys exist only in temporary files with restrictive permissions for the session lifetime.
- **Bounded remote commands**: SSH operations use connection and command timeouts.
- **Input validation and quoting**: SSH targets, container/image names, repository paths, environment keys, and other command inputs are validated; values passed to the remote shell are shell-quoted.
- **Secret-safe runtime injection**: secret environment values are sent over SSH stdin to a temporary remote env file instead of being embedded in logged command strings.
- **Log redaction**: known decrypted secret values are removed from surfaced runtime errors and container logs.

## Network Exposure

Docker Compose binds the management UI/API to `127.0.0.1:5150` by default as defense in depth. If exposing Moonships beyond localhost, terminate TLS at a trusted reverse proxy or use a private VPN, use unique strong `JWT_SECRET` and `ENCRYPTION_KEY` values, and pin remote SSH host fingerprints from a trusted source.

## Known Security Limitations

- v0.2 authenticates users but does not yet implement organization-level RBAC/multi-tenant authorization.
- Trust-on-first-use SSH discovery is weaker than explicit fingerprint pinning.
- Docker access on a target host is highly privileged; the configured deploy user should be treated as privileged on that target.
- AES-GCM nonces provide encryption safety when unique; they are not an application-level replay-prevention mechanism.
