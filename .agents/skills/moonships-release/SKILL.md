---
name: moonships-release
description: Release readiness checklist and verification protocols for Moonships
---

# Moonships Release Skill

## Release Readiness Protocol
Before tagging a release or closing a milestone, run the following verification checklist:

1. **Compilation & Linting**:
   - `cargo fmt --check` passes without reformatting needed.
   - `cargo check --all-targets --all-features` succeeds with 0 errors.
   - `cargo clippy --all-targets --all-features -- -D warnings` produces 0 warnings.
2. **Automated Test Suite**:
   - `cargo test` passes 100% of unit, service, worker, and API tests.
3. **Database & Migrations**:
   - Fresh migration test on an empty SQLite database succeeds.
   - Durability test: persistent worker queue survives application restart without losing pending tasks.
4. **Security Audit**:
   - Zero hardcoded passwords, tokens, or plaintext keys.
   - AES-256-GCM encryption verified for all stored secrets and SSH keys.
   - Command injection and path traversal tests pass.
5. **Container & Deployment**:
   - Multi-stage `Dockerfile` builds cleanly.
   - `docker-compose.yml` config passes validation (`docker compose config`).
6. **Documentation & Changelog**:
   - `README.md` accurately reflects current capabilities and SQLite-first architecture.
   - `CHANGELOG.md` records all changes under the target release tag.
   - All ADRs in `docs/adr/` are committed.
