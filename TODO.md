# MOONSHIPS — Task Index (TODO)

> **Notice**: GitHub Issues on repository [`bjo163/mwx-ships`](https://github.com/bjo163/mwx-ships/issues) are the canonical source of truth for task tracking. This document serves strictly as an index referencing verified GitHub Issue IDs.

---

## Phase 0: Foundation (`M0 Foundation`)

- [x] #1 Initialize Moonships Loco project scaffold and directory layout
- [ ] #2 Configure SQLite database engine and PRAGMA reliability settings
- [ ] #3 Configure persistent SQLite worker queue
- [x] #4 Configure Antigravity project rules and custom agents
- [x] #5 Configure Antigravity engineering skills
- [ ] #6 Configure GitHub Actions CI workflows and Dependabot
- [ ] #7 Configure repository governance, templates, and guidelines

## Phase 1: SQLite Domain Models (`M1 SQLite Domain`)

- [ ] #8 Implement Server model and migrations
- [ ] #9 Implement Project and Environment models and migrations
- [ ] #10 Implement Application model and migrations
- [ ] #11 Implement EnvironmentVariable model and secret storage
- [ ] #12 Implement Domain model and routing configuration
- [ ] #13 Implement Deployment and DeploymentLog models

## Phase 2: Remote Infrastructure Services (`M2 Remote Infrastructure`)

- [ ] #14 Implement CryptoService for AES-256-GCM secret encryption
- [ ] #15 Implement GitService for generic repository operations
- [ ] #16 Implement SshService for remote server access
- [ ] #17 Implement DockerService for remote container lifecycle
- [ ] #18 Implement ProxyService for Traefik label generation
- [ ] #19 Implement Server Preflight inspection endpoint

## Phase 3: Deployment Engine (`M3 Deployment Engine`)

- [ ] #20 Implement DeploymentService orchestration engine
- [ ] #21 Implement DeploymentWorker with persistent SQLite queue
- [ ] #22 Implement deployment state machine and transition persistence
- [ ] #23 Implement deployment concurrency locking (single active deployment)
- [ ] #24 Implement structured deployment logging stream
- [ ] #25 Implement HTTP healthcheck verification loop

## Phase 4: Modern Dashboard (`M4 Dashboard`)

- [ ] #26 Implement React Single Page Dashboard application
- [ ] #27 Implement real-time deployment log viewer and status monitor

## Phase 5: Security & Verification (`M5 Security`)

- [ ] #28 Implement security boundary tests (injection, traversal, secret masking)

## Phase 6: Documentation & Operations (`M6 Documentation`)

- [ ] #29 Implement SQLite backup and disaster recovery procedures
- [ ] #30 Author comprehensive documentation tree and ADRs

## Future Scale Phases

- [ ] #31 Design PostgreSQL scale adapter (`M7 PostgreSQL Adapter`)
- [ ] #32 Plan webhooks, Compose, rollback, and multi-node clustering (`M8 Advanced Platform`)
