# Security Architecture & Policies

Moonships manages privileged application lifecycle operations on remote Docker hosts. The control plane, database/queue, Git providers, SSH boundary, organization authorization, and target runtime are separate trust zones.

## 1. Authentication & Authorization

`GET /api/health` remains public for liveness. Operational routes require an authenticated principal.

Supported principals:
- JWT sessions with revocable `session_version`;
- organization API tokens stored only as hashes, with explicit scopes and optional expiry.

Organization roles are **Owner, Admin, Deployer, Viewer**. Authorization resolves the organization owning the requested project/server/application resource and fails closed on cross-organization access. Sensitive organization actions require explicit confirmation and are audit-attributed.

## 2. Secrets at Rest

Moonships protects SSH keys, application secrets, Git/provider credentials, registry credentials, and optional encrypted backup exports with AES-256-GCM and randomized nonces. Production requires an operator-provided 32-byte / 64-hex-character `ENCRYPTION_KEY`.

API token plaintext is reveal-once and only a hash is persisted.

## 3. Secret Transfer

Runtime secrets, private-registry passwords, and HTTPS Git credentials are not placed in command-line arguments or repository URLs. They are transferred over SSH stdin into restrictive temporary files or stdin-backed authentication mechanisms such as `GIT_ASKPASS` / `docker login --password-stdin`. Known plaintext secret values are included in deployment redaction inputs.

## 4. SSH Host Identity

- SSH uses strict host-key checking with an isolated `known_hosts` file.
- Operators may pin a SHA256 host fingerprint from an out-of-band trusted source.
- Without explicit pinning, discovery is trust-on-first-use and must not be treated as equivalent to out-of-band verification.
- SSH connection/command durations are bounded.

## 5. Remote Command Boundary

Git/Docker/Compose/Traefik workflows require target-side shell composition. Moonships validates inserted values (SSH targets, names, paths, refs, domains, environment keys, registry hosts/usernames) and shell-quotes allowed values. Secrets are passed separately rather than interpolated.

## 6. Control-Plane Privilege

Application workloads execute on selected remote servers. The production Moonships container does **not** mount the host Docker socket and does not need local Docker-daemon privilege.

## 7. Concurrency & Replay

Moonships uses layered database coordination:
- application-level conditional deployment ownership;
- random execution-token leases with periodic heartbeat and stale reclaim;
- lease-token checks on every phase transition;
- signed Git-provider webhook verification;
- unique delivery/source-intent keys for replay/deduplication.

This is tested on SQLite and PostgreSQL. PostgreSQL is required when multiple writable control-plane workers share state.

## 8. Administrative Audit & Abuse Controls

Administrative/destructive actions append immutable audit records containing actor, organization, resource, outcome, request correlation, and bounded metadata. Login and sensitive-action rate limits are keyed by hashed identifiers so raw credential material is not used as rate-limit storage.

## 9. Database Modes

SQLite is the single-node default. PostgreSQL is the optional multi-worker scale mode. The SQLite→PostgreSQL migration tool copies encrypted values as ciphertext and verifies schema/row/checksum/FK parity; the destination therefore requires the same `ENCRYPTION_KEY`.

PostgreSQL HA, transport policy, and database-provider backup operations remain operator responsibilities.


## 10. v1 Threat Model & Release Security

The GA threat model, P0/P1 release-blocking definitions, trust-zone analysis, residual risks, and supply-chain controls are maintained in [threat-model-v1.md](threat-model-v1.md).

The release pipeline for v0.9+:
- builds and pushes one image digest;
- generates BuildKit SBOM/provenance metadata;
- creates a signed GitHub/Sigstore build-provenance attestation for that digest;
- generates an SPDX JSON SBOM and release checksum;
- pulls and healthchecks the exact published digest before tagging/releasing.

The checked-in dependency policy requires third-party packages to declare license metadata and requires resolved Git dependencies to be pinned to a concrete commit.
