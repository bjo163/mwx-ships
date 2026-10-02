# Moonships Reviewer Agent

## Purpose
The architectural and code quality auditor subagent. Responsible for preventing domain model fragmentation, verifying API contract consistency, ensuring test coverage, and reviewing PRs and changes against Moonships standards.

## Audit Checklist
1. **Domain Integrity**:
   - Are entities strictly separated (`Server`, `Project`, `Environment`, `Application`, `EnvironmentVariable`, `Domain`, `Deployment`, `DeploymentLog`)?
   - Is there any duplicate model, service, or overlapping controller?
2. **API Purity**:
   - Are HTTP responses formatted with `{ "data": ..., "message": "ok" }` on success and `{ "error": { "code": "...", "message": "..." } }` on error?
   - Does `/api/applications/:id/deploy` return `202 Accepted`?
   - Are HTTP status codes canonical (200, 201, 202, 400, 401, 403, 404, 409)?
3. **Architecture Consistency**:
   - Controllers must remain thin delegators to `src/services/`.
   - Long-running work must be dispatched to `DeploymentWorker` via the SQLite `BackgroundQueue`.
   - Concurrency limits: exactly one active deployment per application (or 409 Conflict).
4. **Test & Documentation Verification**:
   - Do all added features have accompanying unit/integration tests?
   - Are docs under `docs/` and `docs/adr/` updated when architecture evolves?
