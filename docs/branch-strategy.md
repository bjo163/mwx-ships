# Moonships Branch Strategy

Moonships deliberately keeps the canonical repository to **two working branches**.

## `dev` — integration

- All normal product, test, documentation, dependency, and CI work is committed directly to `dev`.
- Every push runs CI, Docker validation/smoke tests, and security checks.
- The branch may contain unfinished work, but it must remain buildable.
- External contributors should work from forks and target `dev`; they should not create additional branches in the canonical repository.

## `main` — production

- `main` represents the latest production-approved code.
- The only normal promotion path is a validated fast-forward promotion from `dev` to `main`.
- Version/changelog changes are prepared on `dev`.
- Release automation runs from `main` and publishes only when the root Cargo package version changes.

## No feature/release/hotfix branches

Do not create `feature/*`, `fix/*`, `release/*`, or dependency-update branches in the canonical repository.

For urgent production fixes:
1. fix directly on `dev`;
2. run the complete release gates;
3. fast-forward `main` to that verified `dev` commit;
4. publish the patch release from `main`.

This intentionally trades parallel branch fan-out for a simple, highly visible integration stream.

## Dependency maintenance

Automated dependency PR branches are disabled. Dependency upgrades are reviewed and batched directly on `dev`, then validated by:
- `cargo check/test/clippy`;
- frontend build/tests;
- advisory + secret scanning;
- production container build/smoke test.

## Repository settings

The repository default branch should be `main`. Stale historical branches should be deleted after the migration. Branch protection/rulesets should prevent direct production changes to `main` except controlled administration and GitHub Actions release tagging.
