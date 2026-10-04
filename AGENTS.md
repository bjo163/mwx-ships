# MOONSHIPS — Agent Rules & Repository Policy

Moonships is an independent self-hosted Mini-PaaS / deployment control plane built with Loco.rs, SQLite-first, SeaORM, Tokio, Docker, and React.

> Deploy. Control. Own your infrastructure.

## Permanent Repository Rules

1. **SQLite-first**: SQLite is the primary and canonical database for MVP development, CI, testing, and initial production deployment. No PostgreSQL or Redis is required for MVP.
2. **PostgreSQL later**: Design models and queries with standard SeaORM conventions so a PostgreSQL adapter can be introduced seamlessly in future phases without rewriting domain logic.
3. **Loco current API only**: Use only verified, actual methods and types from Loco.rs v1.1/v1.2. Never invent APIs, CLI arguments, or config fields.
4. **No invented APIs**: All internal services, worker queues, and REST routes must be backed by real code, not placeholders or fictional helper functions.
5. **No fake success**: Never report a test, migration, build, backup, or GitHub issue as completed unless actually executed and verified.
6. **No secret leakage**: Never log raw secrets, private SSH keys, or environment tokens. Encrypt secrets at rest using AES-GCM-256 (`ENCRYPTION_KEY`). Never commit secrets to Git.
7. **No unsafe shell interpolation**: Never concatenate unvalidated user input into shell commands or Docker CLI invocations. When a remote shell is required, validate inputs and shell-quote every inserted value through the shared safe execution wrapper.
8. **Thin controllers**: Controllers only handle HTTP parsing, parameter validation, and status dispatch.
9. **Business logic in services**: All orchestration and business logic live in `src/services/` (`GitService`, `SshService`, `DockerService`, `ProxyService`, `DeploymentService`, `CryptoService`).
10. **Deployment in workers**: Deployments run asynchronously in `src/workers/deployment.rs` via Loco's persistent `BackgroundQueue` (SQLite backend). Controllers return `202 Accepted`.
11. **Tests required**: Models, services, workers, security boundaries, and API endpoints must have automated tests using SQLite in-memory or file databases.
12. **Docs required**: Keep architectural decision records (ADRs), user manuals, API specs, and runbooks updated under `docs/`.
13. **GitHub issues required**: GitHub Issues are the single source of truth for task tracking. Keep `TODO.md` in sync as an index referencing real issue IDs.
14. **No duplicate domain concepts**: Maintain clean, non-overlapping models: `Server`, `Project`, `Environment`, `Application`, `EnvironmentVariable`, `Domain`, `Deployment`, `DeploymentLog`.
15. **Two-branch policy**: Canonical development uses only `dev` and `main`. Work directly on `dev`; promote only a green `dev -> main` PR for production. Do not create feature/release/hotfix/dependency branches in the canonical repository.
16. **Release-from-main only**: `main` is production. Version/changelog preparation happens on `dev`; the release workflow publishes from `main` only when the package version changes.

## Code & Structure Conventions

- **Models**: SeaORM entities live in `src/models/_entities/` and domain logic in `src/models/`. Foreign and primary keys are `i64`.
- **Services**: Pure business logic modules under `src/services/`.
- **Workers**: Job definitions implementing `BackgroundWorker<Args>` in `src/workers/`.
- **Controllers**: Grouped routes registered in `src/app.rs`.
- **Queue**: Loco `BackgroundQueue` with SQLite backend for durability across crashes.
- **Errors**: Return `loco_rs::Result<T>` and serialize standard error envelopes: `{ "error": { "code": "...", "message": "..." } }`.
- **Success Responses**: Serialize standard envelopes: `{ "data": ..., "message": "ok" }`.
