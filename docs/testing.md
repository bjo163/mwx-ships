# Testing Strategy & Quality Assurance

Moonships enforces automated test coverage across models, services, workers, security boundaries, and migrations.

## Test Categories

### 1. Service Unit Tests (`tests/services/`)
- **`crypto`**: AES-256-GCM encryption/decryption roundtrips, randomized nonces, and tamper-resistance checks.
- **`git`**: Sanitization of Git URLs and branch names, preventing shell command injection.
- **`docker`**: Validation of container names and image tags, preventing forbidden characters.
- **`proxy`**: RFC 1123 hostname compliance and dynamic Traefik labels generation.

### 2. Model & State Tests (`tests/models/`)
- Safe masking of environment variables and secrets (`to_safe`).
- Automatic slug generation for applications and projects.
- State transitions and validation.

### 3. Worker & Async Tests (`tests/workers/`)
- Persistent SQLite worker queue dispatch.
- Durability across process restarts.

### 4. Running the Tests

To run the complete test suite:
```bash
# Run tests with controlled concurrency for Windows stability
cargo test -j 2

# Run service tests specifically
cargo test services -j 2
```
