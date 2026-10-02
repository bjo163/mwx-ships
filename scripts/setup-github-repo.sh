#!/usr/bin/env bash
set -euo pipefail

REPO="${1:-bjo163/mwx-ships}"
echo "Configuring GitHub Milestones for $REPO..."

milestones=(
  "M0 Foundation:Workspace setup, CI, Loco configuration, SQLite setup"
  "M1 SQLite Domain:SeaORM models, migrations, and SQLite entities"
  "M2 Remote Infrastructure:SSH, Git, Docker, and Proxy services"
  "M3 Deployment Engine:Deployment worker, persistent queue, state machine, logging, healthcheck"
  "M4 Dashboard:React SPA dashboard, real-time log viewers, application control panel"
  "M5 Security:Secret encryption, input validation, authentication guards"
  "M6 Documentation:Architecture docs, ADRs, operations manuals, backup procedures"
  "M7 PostgreSQL Adapter:Future scale adapter design and contracts"
  "M8 Advanced Platform:Webhooks, Compose, zero-downtime, multi-node"
)

for m in "${milestones[@]}"; do
  title="${m%%:*}"
  desc="${m#*:}"
  echo "Creating milestone: $title"
  gh api "repos/$REPO/milestones" -f title="$title" -f description="$desc" 2>/dev/null || true
done

echo "Configuring GitHub Labels for $REPO..."
labels=(
  "area/backend:0052cc:Backend logic, controllers, services"
  "area/frontend:1d76db:React UI, components, dashboard"
  "area/database:5319e7:SQLite, SeaORM, migrations"
  "area/docker:0db7ed:Docker engine, container lifecycle"
  "area/git:f05032:Git clone, checkout, branch operations"
  "area/ssh:333333:SSH connectivity, key management"
  "area/security:d93f0b:Encryption, auth, input sanitization"
  "area/devops:bfdadc:CI/CD, build tools, packaging"
  "area/docs:0075ca:Documentation, ADRs, runbooks"
  "type/feature:a2eeef:New feature or capability"
  "type/bug:d73a4a:Defect or issue"
  "type/chore:cfd3d7:Maintenance or refactoring"
  "type/docs:0075ca:Documentation update"
  "type/security:e11d48:Security hardening or advisory"
  "priority/high:b60205:High priority task"
  "priority/medium:fbca04:Medium priority task"
  "priority/low:0e8a16:Low priority task"
  "phase/mvp:0e8a16:Required for SQLite-First MVP"
  "phase/postgres:6f42c1:PostgreSQL scale adapter phase"
  "phase/future:d4c5f9:Future platform enhancements"
)

for l in "${labels[@]}"; do
  IFS=":" read -r name color desc <<< "$l"
  echo "Creating label: $name"
  gh label create "$name" --color "$color" --description "$desc" --repo "$REPO" --force || true
done

echo "GitHub setup complete!"
