# Moonships v1 Threat Model

## Security objective

Moonships is a privileged deployment control plane. Its primary security objective is to prevent an attacker, compromised user, or failed integration from crossing organization, secret, control-plane, or remote-host boundaries without authorization and auditability.

## Assets

Critical assets include SSH private keys, application secrets, Git/registry credentials, organization API tokens, sessions, deployment revisions, audit history, control-plane state, remote runtimes, backup exports, and release artifacts.

## Trust zones

1. Internet and browser.
2. Control-plane HTTP API.
3. Database and persistent queue.
4. External Git providers.
5. SSH transport boundary.
6. Target Docker/Compose runtime and managed ingress.
7. Container registries.
8. Backup storage.
9. CI and release system.

## Principal threats and controls

### Credential disclosure

Controls include AES-256-GCM at rest, reveal-once API tokens, secret-safe DTOs, SSH stdin transfer, restrictive temporary files, log redaction, and CI secret scanning.

Residual risk: a fully compromised control-plane process can access secrets required to perform its duties. Host/process hardening remains an operator responsibility.

### Cross-organization privilege escalation

Controls include organization ownership boundaries, Owner/Admin/Deployer/Viewer policy, scoped hashed API tokens, revocable JWT session versions, audit attribution, and fail-closed authorization tests.

### Remote command injection

Controls include strict validators, shared shell quoting, stdin transfer for large or secret configuration, bounded SSH timeouts, target host-key verification, and no control-plane Docker socket.

### Webhook forgery and replay

Controls include provider-specific HMAC/token verification, persistent delivery ledger, commit-intent dedupe, encrypted provider credentials, and disabled webhook integrations on derived preview applications.

### Duplicate or stale worker execution

Controls include atomic application ownership, execution-token leases, heartbeat renewal, token-qualified transitions, stale lease reclaim, and concurrency tests.

### Bad deployment replaces healthy traffic

Controls include immutable revisions, candidate runtime, health-before-switch, atomic ingress switch, known-good state, and rollback.

### Backup or database loss

Controls include SQLite online backup, retention and verification, encrypted exports, automated restore drills, explicit PostgreSQL operator backup boundaries, and backup-before-upgrade policy.

### Supply-chain compromise

Controls include locked dependencies, advisory scan, dependency source/license metadata gate, secret scan, non-root runtime image, release image verification by registry digest, SBOM/provenance generation, and GitHub/Sigstore build attestation.

## Severity policy

A release candidate cannot be promoted to v1 with any unresolved:

- P0: known exploitable path to remote code execution, secret disclosure, authentication bypass, cross-organization control, irreversible control-plane data loss, or release artifact substitution.
- P1: reproducible deployment corruption, rollback failure, stale-worker double execution, backup/restore failure, authorization-policy bypass with meaningful impact, or supported upgrade data loss.

P2/P3 issues may ship only when documented, bounded, and not contradictory to the v1 support contract.

## Out of scope for v1

- Kubernetes or service-mesh orchestration.
- Host OS patch management.
- PostgreSQL HA implementation.
- DNS provider automation.
- Enterprise SSO or federation.
- Application database/data-volume backups.
- Protection against a fully compromised root account on a target host.
