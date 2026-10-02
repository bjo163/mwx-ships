#!/usr/bin/env bash
set -euo pipefail

REPO="${1:-bjo163/mwx-ships}"
echo "Creating initial backlog on $REPO..."

issues=(
  "Initialize Moonships Loco project scaffold and directory layout|Set up core Loco scaffold|M0 Foundation|area/backend,type/chore,priority/high,phase/mvp"
  "Configure SQLite database engine and PRAGMA reliability settings|Establish persistent SQLite configuration|M0 Foundation|area/database,type/feature,priority/high,phase/mvp"
  "Configure persistent SQLite worker queue|Set up Loco BackgroundQueue with SQLite backend|M0 Foundation|area/backend,type/feature,priority/high,phase/mvp"
  "Configure Antigravity project rules and custom agents|Establish AGENTS.md and custom agents|M0 Foundation|area/devops,type/chore,priority/medium,phase/mvp"
  "Configure Antigravity engineering skills|Define .agents/skills for development and testing|M0 Foundation|area/devops,type/chore,priority/medium,phase/mvp"
  "Configure GitHub Actions CI workflows and Dependabot|Implement CI pipeline|M0 Foundation|area/devops,type/chore,priority/high,phase/mvp"
  "Configure repository governance templates and guidelines|Add issue and PR templates|M0 Foundation|area/docs,type/chore,priority/medium,phase/mvp"
  "Implement Server model and migrations|Define Server entity|M1 SQLite Domain|area/database,type/feature,priority/high,phase/mvp"
  "Implement Project and Environment models and migrations|Define Project and Environment entities|M1 SQLite Domain|area/database,type/feature,priority/high,phase/mvp"
  "Implement Application model and migrations|Define Application entity|M1 SQLite Domain|area/database,type/feature,priority/high,phase/mvp"
  "Implement EnvironmentVariable model and secret storage|Store encrypted environment variables|M1 SQLite Domain|area/database,type/feature,priority/high,phase/mvp"
  "Implement Domain model and routing configuration|Manage hostnames and ports for Traefik|M1 SQLite Domain|area/database,type/feature,priority/medium,phase/mvp"
  "Implement Deployment and DeploymentLog models|Track deployment transitions and logs|M1 SQLite Domain|area/database,type/feature,priority/high,phase/mvp"
  "Implement CryptoService for AES-256-GCM secret encryption|Encrypt secrets using ENCRYPTION_KEY|M2 Remote Infrastructure|area/security,type/feature,priority/high,phase/mvp"
  "Implement GitService for generic repository operations|Clone and inspect git repositories|M2 Remote Infrastructure|area/git,type/feature,priority/high,phase/mvp"
  "Implement SshService for remote server access|Manage SSH connections and commands|M2 Remote Infrastructure|area/ssh,type/feature,priority/high,phase/mvp"
  "Implement DockerService for remote container lifecycle|Structured Docker operations|M2 Remote Infrastructure|area/docker,type/feature,priority/high,phase/mvp"
  "Implement ProxyService for Traefik label generation|Generate Traefik labels|M2 Remote Infrastructure|area/backend,type/feature,priority/medium,phase/mvp"
  "Implement Server Preflight inspection endpoint|Inspect remote Linux machine|M2 Remote Infrastructure|area/backend,type/feature,priority/medium,phase/mvp"
  "Implement DeploymentService orchestration engine|Coordinate Git, SSH, Docker, and Proxy|M3 Deployment Engine|area/backend,type/feature,priority/high,phase/mvp"
  "Implement DeploymentWorker with persistent SQLite queue|Execute async deployment pipeline|M3 Deployment Engine|area/backend,type/feature,priority/high,phase/mvp"
  "Implement deployment state machine and transition persistence|Handle status transitions|M3 Deployment Engine|area/backend,type/feature,priority/high,phase/mvp"
  "Implement deployment concurrency locking (single active deployment)|Prevent overlapping deployments|M3 Deployment Engine|area/backend,type/feature,priority/high,phase/mvp"
  "Implement structured deployment logging stream|Persist sequence-ordered logs|M3 Deployment Engine|area/backend,type/feature,priority/high,phase/mvp"
  "Implement HTTP healthcheck verification loop|Verify application health before success|M3 Deployment Engine|area/backend,type/feature,priority/medium,phase/mvp"
  "Implement React Single Page Dashboard application|Modern responsive dashboard UI|M4 Dashboard|area/frontend,type/feature,priority/high,phase/mvp"
  "Implement real-time deployment log viewer and status monitor|Interactive streaming log console|M4 Dashboard|area/frontend,type/feature,priority/high,phase/mvp"
  "Implement security boundary tests (injection, traversal, secret masking)|Security test suite|M5 Security|area/security,type/security,priority/high,phase/mvp"
  "Implement SQLite backup and disaster recovery procedures|Backup and restore runbooks|M6 Documentation|area/database,type/feature,priority/high,phase/mvp"
  "Author comprehensive documentation tree and ADRs|Full docs and ADRs|M6 Documentation|area/docs,type/docs,priority/high,phase/mvp"
  "Design PostgreSQL scale adapter|PostgreSQL future architecture|M7 PostgreSQL Adapter|area/database,type/feature,priority/low,phase/postgres"
  "Plan webhooks, Compose, rollback, and multi-node clustering|Future platform roadmap|M8 Advanced Platform|area/backend,type/feature,priority/low,phase/future"
)

for item in "${issues[@]}"; do
  IFS="|" read -r title body milestone labels <<< "$item"
  echo "Creating: $title"
  gh issue create --repo "$REPO" --title "$title" --body "$body" --milestone "$milestone" --label "$labels"
done

echo "All issues created on $REPO!"
