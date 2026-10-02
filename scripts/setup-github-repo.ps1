# Setup GitHub Milestones and Labels for Moonships
param(
    [string]$Repo = "bjo163/mwx-ships"
)

Write-Host "Configuring GitHub Milestones for $Repo..." -ForegroundColor Cyan

$milestones = @(
    @{ title = "M0 Foundation"; desc = "Workspace setup, CI, Loco configuration, SQLite setup" },
    @{ title = "M1 SQLite Domain"; desc = "SeaORM models, migrations, and SQLite entities" },
    @{ title = "M2 Remote Infrastructure"; desc = "SSH, Git, Docker, and Proxy services" },
    @{ title = "M3 Deployment Engine"; desc = "Deployment worker, persistent queue, state machine, logging, healthcheck" },
    @{ title = "M4 Dashboard"; desc = "React SPA dashboard, real-time log viewers, application control panel" },
    @{ title = "M5 Security"; desc = "Secret encryption, input validation, authentication guards" },
    @{ title = "M6 Documentation"; desc = "Architecture docs, ADRs, operations manuals, backup procedures" },
    @{ title = "M7 PostgreSQL Adapter"; desc = "Future scale adapter design and contracts" },
    @{ title = "M8 Advanced Platform"; desc = "Webhooks, Compose, zero-downtime, multi-node" }
)

foreach ($m in $milestones) {
    Write-Host "Creating milestone: $($m.title)"
    gh api "repos/$Repo/milestones" -f title="$($m.title)" -f description="$($m.desc)" 2>$null
}

Write-Host "Configuring GitHub Labels for $Repo..." -ForegroundColor Cyan

$labels = @(
    @{ name = "area/backend"; color = "0052cc"; desc = "Backend logic, controllers, services" },
    @{ name = "area/frontend"; color = "1d76db"; desc = "React UI, components, dashboard" },
    @{ name = "area/database"; color = "5319e7"; desc = "SQLite, SeaORM, migrations" },
    @{ name = "area/docker"; color = "0db7ed"; desc = "Docker engine, container lifecycle" },
    @{ name = "area/git"; color = "f05032"; desc = "Git clone, checkout, branch operations" },
    @{ name = "area/ssh"; color = "333333"; desc = "SSH connectivity, key management" },
    @{ name = "area/security"; color = "d93f0b"; desc = "Encryption, auth, input sanitization" },
    @{ name = "area/devops"; color = "bfdadc"; desc = "CI/CD, build tools, packaging" },
    @{ name = "area/docs"; color = "0075ca"; desc = "Documentation, ADRs, runbooks" },
    
    @{ name = "type/feature"; color = "a2eeef"; desc = "New feature or capability" },
    @{ name = "type/bug"; color = "d73a4a"; desc = "Defect or issue" },
    @{ name = "type/chore"; color = "cfd3d7"; desc = "Maintenance or refactoring" },
    @{ name = "type/docs"; color = "0075ca"; desc = "Documentation update" },
    @{ name = "type/security"; color = "e11d48"; desc = "Security hardening or advisory" },
    
    @{ name = "priority/high"; color = "b60205"; desc = "High priority task" },
    @{ name = "priority/medium"; color = "fbca04"; desc = "Medium priority task" },
    @{ name = "priority/low"; color = "0e8a16"; desc = "Low priority task" },
    
    @{ name = "phase/mvp"; color = "0e8a16"; desc = "Required for SQLite-First MVP" },
    @{ name = "phase/postgres"; color = "6f42c1"; desc = "PostgreSQL scale adapter phase" },
    @{ name = "phase/future"; color = "d4c5f9"; desc = "Future platform enhancements" }
)

foreach ($l in $labels) {
    Write-Host "Creating label: $($l.name)"
    gh label create "$($l.name)" --color "$($l.color)" --description "$($l.desc)" --repo "$Repo" --force
}

Write-Host "GitHub Milestones and Labels configured successfully!" -ForegroundColor Green
