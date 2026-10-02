# MOONSHIPS — Task Index (TODO)

> **Notice**: GitHub Issues on repository [`bjo163/mwx-ships`](https://github.com/bjo163/mwx-ships/issues) are the canonical source of truth for task tracking. This document serves strictly as an index referencing verified GitHub Issue IDs.

---

## Phase 0: Foundation (`M0 Foundation`)

- [x] #1 Initialize Moonships Loco project scaffold and directory layout
- [x] #2 Configure SQLite database engine and PRAGMA reliability settings
- [x] #3 Configure persistent SQLite worker queue
- [x] #4 Configure Antigravity project rules and custom agents
- [x] #5 Configure Antigravity engineering skills
- [x] #6 Configure GitHub Actions CI workflows and Dependabot
- [x] #7 Configure repository governance, templates, and guidelines

## Phase 1: SQLite Domain Models (`M1 SQLite Domain`)

- [x] #8 Implement Server model and migrations
- [x] #9 Implement Project and Environment models and migrations
- [x] #10 Implement Application model and migrations
- [x] #11 Implement EnvironmentVariable model and secret storage
- [x] #12 Implement Domain model and routing configuration
- [x] #13 Implement Deployment and DeploymentLog models

## Phase 2: Remote Infrastructure Services (`M2 Remote Infrastructure`)

- [x] #14 Implement CryptoService for AES-256-GCM secret encryption
- [x] #15 Implement GitService for generic repository operations
- [x] #16 Implement SshService for remote server access
- [x] #17 Implement DockerService for remote container lifecycle
- [x] #18 Implement ProxyService for Traefik label generation
- [x] #19 Implement Server Preflight inspection endpoint

## Phase 3: Deployment Engine (`M3 Deployment Engine`)

- [x] #20 Implement DeploymentService orchestration engine
- [x] #21 Implement DeploymentWorker with persistent SQLite queue
- [x] #22 Implement deployment state machine and transition persistence
- [x] #23 Implement deployment concurrency locking (single active deployment)
- [x] #24 Implement structured deployment logging stream
- [x] #25 Implement HTTP healthcheck verification loop

## Phase 4: Modern Dashboard (`M4 Dashboard`)

- [x] #26 Implement React Single Page Dashboard application
- [x] #27 Implement real-time deployment log viewer and status monitor

## Phase 5: Security & Verification (`M5 Security`)

- [x] #28 Implement security boundary tests (injection, traversal, secret masking)

## Phase 6: Documentation & Operations (`M6 Documentation`)

- [x] #29 Implement SQLite backup and disaster recovery procedures
- [x] #30 Author comprehensive documentation tree and ADRs

## Future Scale Phases

- [ ] #31 Design PostgreSQL scale adapter (`M7 PostgreSQL Adapter`)
- [ ] #32 Plan webhooks, Compose, rollback, and multi-node clustering (`M8 Advanced Platform`)
