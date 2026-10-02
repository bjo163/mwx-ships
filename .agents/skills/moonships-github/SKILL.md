---
name: moonships-github
description: GitHub issue, milestone, and label management procedures for Moonships
---

# Moonships GitHub Skill

## Canonical Source of Truth
GitHub Issues are the canonical task backlog for the project. `TODO.md` is strictly an index that mirrors verified GitHub Issue IDs.

## Workflow Rules
1. **Inspect First**: Before creating an issue, inspect existing issues using `gh issue list` to prevent duplicate backlog items.
2. **Never Invent Numbers**: Only reference real issue numbers returned by GitHub CLI or API.
3. **Milestone Organization**:
   - `M0 Foundation`: Workspace, CI, Loco configuration, SQLite setup.
   - `M1 SQLite Domain`: SeaORM models, entities, and migrations.
   - `M2 Remote Infrastructure`: Git, SSH, Docker, and Proxy services.
   - `M3 Deployment Engine`: Deployment worker, queue, state machine, logging, healthcheck.
   - `M4 Dashboard`: React SPA dashboard, real-time log viewers, application management.
   - `M5 Security`: Secret encryption, input validation, authentication guards.
   - `M6 Documentation`: Architecture docs, ADRs, operations manuals, backup procedures.
   - `M7 PostgreSQL Adapter`: Future scale adapter design and contracts.
   - `M8 Advanced Platform`: Webhooks, Compose, zero-downtime, multi-node.
4. **Label Taxonomy**:
   - Areas: `area/backend`, `area/frontend`, `area/database`, `area/docker`, `area/git`, `area/ssh`, `area/security`, `area/devops`, `area/docs`
   - Types: `type/feature`, `type/bug`, `type/chore`, `type/docs`, `type/security`
   - Priorities: `priority/high`, `priority/medium`, `priority/low`
   - Phases: `phase/mvp`, `phase/postgres`, `phase/future`
