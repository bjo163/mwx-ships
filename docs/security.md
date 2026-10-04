# Security Architecture & Policies

Moonships manages privileged Docker operations on remote target machines, so v0.2 treats the control plane, SSH boundary, secrets, and target runtime as separate trust zones.

## 1. Authentication Boundary

Operational management routes require a valid JWT. `GET /api/health` remains public for liveness checks. Multi-tenant organization RBAC is not yet implemented.

## 2. Secrets At Rest

SSH private keys and environment variables marked secret are encrypted with AES-256-GCM and randomized nonces before database insertion. Production requires an operator-provided 32-byte/64-hex-character `ENCRYPTION_KEY`; there is no repository-known fallback.

## 3. SSH Credentials and Host Identity

- Decrypted SSH private keys are written only to temporary files with restrictive permissions and removed when the session is dropped.
- SSH uses `StrictHostKeyChecking=yes` with an isolated scanned `known_hosts` file.
- A configured SHA256 `known_host_fingerprint` is verified before connecting.
- Without explicit pinning, scanned host-key discovery is trust-on-first-use and should not be treated as equivalent to out-of-band verification.

## 4. Remote Command Safety

v0.2 uses a remote shell over OpenSSH because Git and Docker workflows require compound target-side commands. Safety is enforced by validating SSH targets, container/image names, Docker context/Dockerfile paths, and environment keys; rejecting path traversal and option-style SSH target injection; shell-quoting inserted values; and applying bounded connection/command timeouts.

## 5. Secret Transfer and Log Redaction

Secret environment values are sent through SSH stdin into a temporary target-side env file created under a restrictive umask. They are not embedded into deployment command log strings. Known decrypted secret values are redacted from surfaced runtime failures and container logs.

## 6. Control-Plane Privilege Reduction

Application workloads execute on the selected target server. The production Moonships container no longer mounts `/var/run/docker.sock` and does not need the Docker CLI for application lifecycle operations.

## 7. Concurrency

Only one active deployment may run per application. This prevents overlapping build/replacement operations. This is concurrency control, not cryptographic replay prevention.
