# Security Policy

## Supported Versions

Moonships is pre-GA until v1.0. Security fixes are prioritized for the latest released minor and, when practical, the immediately previous minor.

| Version | Security support |
| --- | --- |
| 0.8.x | Current |
| 0.7.x | Best-effort critical fixes |
| <= 0.6.x | Upgrade required |
| 1.x | Becomes the stable support line at GA |

See [docs/upgrade-policy.md](docs/upgrade-policy.md) for the tested forward-upgrade matrix.

## Reporting a Vulnerability

Do not disclose suspected vulnerabilities in public issues or discussions.

Use GitHub Security Advisories for coordinated private disclosure and include:
- affected Moonships version/image digest;
- vulnerability category;
- reproduction steps;
- impact and trust boundary crossed;
- any known mitigation.

## Security Architecture

The current architecture is documented in [docs/security.md](docs/security.md). The v1 trust-zone analysis, residual risks, severity policy, and release-blocking P0/P1 definitions are in [docs/threat-model-v1.md](docs/threat-model-v1.md).

Core properties include:
- authenticated operational control plane with organization RBAC and scoped API tokens;
- AES-256-GCM secret storage using an operator-provided encryption key;
- remote execution over SSH with strict host verification and bounded commands;
- no control-plane Docker socket dependency;
- secret-safe stdin transfer and log redaction;
- signed/deduplicated Git webhooks;
- atomic deployment ownership plus execution leases;
- immutable revisions, health-before-switch, and rollback;
- tested backup/restore and supported upgrade paths;
- security/advisory/secret/dependency-policy CI gates;
- release SBOM and signed build provenance for v0.9+.

## Operator Responsibilities

Moonships cannot protect against a fully compromised root account on a target host or a compromised host running the control plane. Operators are responsible for OS/network hardening, trusted TLS termination, PostgreSQL HA/backups in scale mode, and custody/rotation of ENCRYPTION_KEY and JWT_SECRET.
