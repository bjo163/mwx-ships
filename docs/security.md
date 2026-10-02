# Security Architecture & Policies

Security is a foundational design requirement for Moonships. Because Moonships manages rootless or privileged Docker execution on remote machines, stringent security policies are enforced across all layers.

## 1. Zero Plaintext Secrets At Rest
- All SSH private keys (`encrypted_private_key` on servers) and sensitive environment variables (`encrypted_value` where `is_secret = true`) are encrypted using **AES-256-GCM** with a randomized 12-byte nonce before database insertion.
- Decryption happens exclusively in-memory during deployment execution.

## 2. Secret Masking in API & Logs
- The REST API endpoint `GET /api/applications/:id/environment` masks secret values to `••••••••`.
- Application secrets and decrypted SSH keys are never written to `deployment_logs`.
- Error messages returned to clients never leak decrypted keys or environment variables.

## 3. Command Injection Prevention
- Moonships strictly avoids string concatenation when executing commands (e.g. `format!("docker run {}", user_input)` is forbidden).
- All arguments are passed as discrete vectors or sanitized through structured validators:
  - Container names: `[a-zA-Z0-9_.-]+` only.
  - Image names: forbid shell metacharacters (`;`, `&`, `|`, `` ` ``, `$`).
  - Git branch names: reject flag prefixes (`-`), path traversal (`..`), spaces, and metacharacters.
  - Git URLs: validated to begin with `https://`, `http://`, `git@`, or `ssh://`.
  - Hostnames: strictly conform to RFC 1123 DNS standards.

## 4. Path Traversal Defenses
- Dockerfile paths and contexts are sanitized to prevent accessing files outside the cloned workspace directory.

## 5. Concurrency & Replay Protection
- Maximum 1 active deployment per application (409 Conflict locks).
- Nonces prevent ciphertext replay attacks.
