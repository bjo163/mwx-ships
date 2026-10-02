# Contributing to Moonships

Thank you for your interest in contributing to **Moonships**!

Moonships is an independent, self-hosted deployment control plane built with Rust, Loco.rs, SQLite-first, and React.

## Core Development Philosophy

1. **SQLite-First**: Moonships is built to run anywhere without requiring PostgreSQL, Redis, or Kubernetes. All PRs must maintain full compatibility with SQLite.
2. **Loco.rs Standards**: Follow established Loco conventions. Use `loco_rs::prelude::*`, place business logic into `src/services/`, keep controllers thin, and delegate asynchronous jobs to `DeploymentWorker` via the SQLite `BackgroundQueue`.
3. **Quality Gates**: Every pull request must pass:
   ```bash
   cargo fmt --check
   cargo check --all-targets --all-features
   cargo test --all-targets --all-features
   cargo clippy --all-targets --all-features -- -D warnings
   ```
4. **Issue-Driven**: Every pull request must link to an active GitHub Issue.

## Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/bjo163/mwx-ships.git
   cd mwx-ships
   ```
2. Set up local configuration:
   ```bash
   cp .env.example .env
   mkdir -p data
   ```
3. Run migrations:
   ```bash
   cargo loco db migrate
   ```
4. Start server and background worker:
   ```bash
   cargo loco start --server-and-worker
   ```

## Pull Request Guidelines

- Fill out the complete pull request template (`.github/pull_request_template.md`).
- Ensure no secret tokens or SSH private keys are ever committed.
- Keep commits concise and descriptive.
