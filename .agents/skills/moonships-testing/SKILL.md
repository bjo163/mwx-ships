---
name: moonships-testing
description: Testing strategies, verification commands, and test suites for Moonships
---

# Moonships Testing Skill

## Testing Philosophy
Moonships enforces comprehensive automated tests that run without external dependencies. SQLite in file or in-memory mode is used exclusively for test suites.

## Quality Gates & Verification Commands
Always run the following commands sequentially:
```bash
# 1. Format Check
cargo fmt --check

# 2. Type Check
cargo check --all-targets --all-features

# 3. Test Suite
cargo test

# 4. Strict Linting
cargo clippy --all-targets --all-features -- -D warnings
```

## Test Dimensions
1. **Model Tests**: Test validation rules, unique constraints, and relationships for `Server`, `Project`, `Environment`, `Application`, `EnvironmentVariable`, `Domain`, `Deployment`, and `DeploymentLog`.
2. **Service Tests**: Test `CryptoService` (encryption/decryption roundtrips), `ProxyService` (Traefik label generation), `GitService` (URL sanitization, clone safety), `DockerService` (command escaping, port parsing).
3. **Worker Tests**: Test `DeploymentWorker` state transitions (`queued` -> `cloning` -> `building` -> `stopping_old` -> `starting_new` -> `healthchecking` -> `success`), handling simulated Git or Docker failures cleanly.
4. **Security Tests**: Validate masking of secrets, rejection of path traversal payloads (`../../etc/passwd`), rejection of command injection attempts (`myapp; rm -rf /`), and rejection of unauthenticated access.
5. **Migration Tests**: Verify fresh migrations on an empty SQLite database file to ensure schema integrity.
