# ADR 0005: Future PostgreSQL Scale Adapter Architecture

## Status
Accepted (Planned Evolution)

## Context
While SQLite is ideal for single-node installations, small teams, and embedded operations, large-scale multi-operator environments with multiple concurrent workers benefit from client-server database architectures like PostgreSQL.

We need a clear architectural strategy to ensure PostgreSQL can be introduced in later phases without rewriting controllers, services, or domain models.

## Decision
We design the domain layer to be database-agnostic using SeaORM abstractions:
1. **No Backend Branching in Services**: Business logic in `src/services/` and controllers must NEVER branch with code like `if postgres { ... } else if sqlite { ... }`.
2. **SeaORM Entity Commonality**: The schema migrations and entity models in `src/models/_entities/` adhere to standard SQL types supported by both SQLite and PostgreSQL (integer primary keys, text, timestamps).
3. **Configuration-Driven Driver**: Future versions will switch drivers based on configuration (`database.uri: postgres://...`) and register the corresponding SeaORM connection pool.

## Consequences
- The SQLite MVP remains clean and uncompromised by early multi-database branching.
- Transitioning to PostgreSQL in Milestone M7 will be a matter of configuration and migration verification rather than architectural refactoring.
