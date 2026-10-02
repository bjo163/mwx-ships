---
name: moonships-development
description: Core development cycle and architecture discipline for the Moonships control plane
---

# Moonships Development Skill

## Overview
This skill guides engineers and agents in executing the disciplined Moonships engineering cycle.

## The Development Loop
Every task must adhere to the 8-step cycle:
1. **Inspect**: Examine existing files, schemas, and git state before touching code.
2. **Plan**: Write a succinct, step-by-step implementation plan.
3. **Implement**: Keep controllers thin, put business logic in `src/services/`, and long-running execution in `src/workers/`.
4. **Compile**: Run `cargo check` and verify type compliance.
5. **Test**: Run unit and integration tests using SQLite test fixtures.
6. **Review**: Check for architectural overlap, secret exposure, or unescaped shell commands.
7. **Document**: Update relevant guides in `docs/` or ADRs if architectural decisions changed.
8. **Update Issue**: Sync progress with the corresponding GitHub issue.

## Non-Negotiable Core Policies
- **SQLite-First**: Development, testing, CI, and initial deployment all rely strictly on SQLite.
- **Loco-Current**: Only use verified APIs provided by the installed version of `loco-rs` and `sea-orm`.
- **Zero Fictional Logic**: Never use `todo!()` or `unimplemented!()` in core deployment workflows.
- **Single Active Deployment**: Applications cannot run concurrent deployments; lock or reject with 409 Conflict.
