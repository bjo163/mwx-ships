# Create initial issue backlog for Moonships
param(
    [string]$Repo = "bjo163/mwx-ships"
)

$issues = @(
    # Foundation (M0 Foundation)
    @{
        title = "Initialize Moonships Loco project scaffold and directory layout"
        body = "Set up the core Loco.rs scaffold with client-side assets and background worker queue configuration."
        milestone = "M0 Foundation"
        labels = "area/backend,type/chore,priority/high,phase/mvp"
    },
    @{
        title = "Configure SQLite database engine and PRAGMA reliability settings"
        body = "Establish persistent SQLite configuration with WAL journaling, busy timeouts, and single-writer boundaries."
        milestone = "M0 Foundation"
        labels = "area/database,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Configure persistent SQLite worker queue"
        body = "Set up Loco BackgroundQueue with SQLite backend ensuring background jobs survive restarts."
        milestone = "M0 Foundation"
        labels = "area/backend,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Configure Antigravity project rules and custom agents"
        body = "Establish AGENTS.md permanent policy and custom agents (.agents/agents/)."
        milestone = "M0 Foundation"
        labels = "area/devops,type/chore,priority/medium,phase/mvp"
    },
    @{
        title = "Configure Antigravity engineering skills"
        body = "Define .agents/skills for development, testing, GitHub sync, and release gates."
        milestone = "M0 Foundation"
        labels = "area/devops,type/chore,priority/medium,phase/mvp"
    },
    @{
        title = "Configure GitHub Actions CI workflows and Dependabot"
        body = "Implement formatting, compilation, testing, clippy, and security linting in CI using SQLite."
        milestone = "M0 Foundation"
        labels = "area/devops,type/chore,priority/high,phase/mvp"
    },
    @{
        title = "Configure repository governance, templates, and guidelines"
        body = "Add issue templates, PR template, CONTRIBUTING.md, CODE_OF_CONDUCT.md, and SECURITY.md."
        milestone = "M0 Foundation"
        labels = "area/docs,type/chore,priority/medium,phase/mvp"
    },

    # Domain (M1 SQLite Domain)
    @{
        title = "Implement Server model and migrations"
        body = "Define Server entity with host, port, username, auth type, encrypted key, fingerprint, and status."
        milestone = "M1 SQLite Domain"
        labels = "area/database,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement Project and Environment models and migrations"
        body = "Define Project and Environment entities for grouping and stage isolation."
        milestone = "M1 SQLite Domain"
        labels = "area/database,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement Application model and migrations"
        body = "Define Application entity with repository, branch, build type, ports, status, and healthcheck."
        milestone = "M1 SQLite Domain"
        labels = "area/database,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement EnvironmentVariable model and secret storage"
        body = "Store encrypted environment variables with secret masking capabilities."
        milestone = "M1 SQLite Domain"
        labels = "area/database,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement Domain model and routing configuration"
        body = "Manage hostnames, ports, and HTTPS flags for Traefik proxy routing."
        milestone = "M1 SQLite Domain"
        labels = "area/database,type/feature,priority/medium,phase/mvp"
    },
    @{
        title = "Implement Deployment and DeploymentLog models"
        body = "Track deployment state transitions, exit codes, error diagnostics, and stream-separated log entries."
        milestone = "M1 SQLite Domain"
        labels = "area/database,type/feature,priority/high,phase/mvp"
    },

    # Infrastructure (M2 Remote Infrastructure)
    @{
        title = "Implement CryptoService for AES-256-GCM secret encryption"
        body = "Encrypt and decrypt SSH private keys and environment secrets using ENCRYPTION_KEY."
        milestone = "M2 Remote Infrastructure"
        labels = "area/security,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement GitService for generic repository operations"
        body = "Clone, fetch, checkout branch/commit, and retrieve current SHA over HTTPS and SSH."
        milestone = "M2 Remote Infrastructure"
        labels = "area/git,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement SshService for remote server access"
        body = "Manage remote SSH connections, command execution, and file transfers."
        milestone = "M2 Remote Infrastructure"
        labels = "area/ssh,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement DockerService for remote container lifecycle"
        body = "Provide structured Docker operations: build, pull, run, stop, restart, inspect, and log retrieval."
        milestone = "M2 Remote Infrastructure"
        labels = "area/docker,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement ProxyService for Traefik label generation"
        body = "Generate Traefik container labels and dynamic routing metadata for applications."
        milestone = "M2 Remote Infrastructure"
        labels = "area/backend,type/feature,priority/medium,phase/mvp"
    },
    @{
        title = "Implement Server Preflight inspection endpoint"
        body = "Inspect remote Linux machine for Docker, disk space, memory, CPU, and user permissions."
        milestone = "M2 Remote Infrastructure"
        labels = "area/backend,type/feature,priority/medium,phase/mvp"
    },

    # Deployment Engine (M3 Deployment Engine)
    @{
        title = "Implement DeploymentService orchestration engine"
        body = "Coordinate GitService, SshService, DockerService, and ProxyService during deployments."
        milestone = "M3 Deployment Engine"
        labels = "area/backend,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement DeploymentWorker with persistent SQLite queue"
        body = "Execute asynchronous deployment pipeline via Loco BackgroundQueue accepting deployment_id."
        milestone = "M3 Deployment Engine"
        labels = "area/backend,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement deployment state machine and transition persistence"
        body = "Handle queued -> cloning -> building -> stopping_old -> starting_new -> healthchecking -> success transitions."
        milestone = "M3 Deployment Engine"
        labels = "area/backend,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement deployment concurrency locking (single active deployment)"
        body = "Prevent overlapping deployments per application with 409 Conflict rejection."
        milestone = "M3 Deployment Engine"
        labels = "area/backend,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement structured deployment logging stream"
        body = "Persist sequence-ordered stdout, stderr, and system logs to DeploymentLog table."
        milestone = "M3 Deployment Engine"
        labels = "area/backend,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement HTTP healthcheck verification loop"
        body = "Poll configured path and port with retry interval and timeout before marking deployment success."
        milestone = "M3 Deployment Engine"
        labels = "area/backend,type/feature,priority/medium,phase/mvp"
    },

    # Dashboard & Frontend (M4 Dashboard)
    @{
        title = "Implement React Single Page Dashboard application"
        body = "Modern responsive dashboard UI for servers, projects, applications, and deployments."
        milestone = "M4 Dashboard"
        labels = "area/frontend,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Implement real-time deployment log viewer and status monitor"
        body = "Interactive streaming log console with auto-scroll and stream filters."
        milestone = "M4 Dashboard"
        labels = "area/frontend,type/feature,priority/high,phase/mvp"
    },

    # Security & Operations (M5 Security & M6 Documentation)
    @{
        title = "Implement security boundary tests (injection, traversal, secret masking)"
        body = "Test command injection resistance, path traversal safeguards, and secret redaction."
        milestone = "M5 Security"
        labels = "area/security,type/security,priority/high,phase/mvp"
    },
    @{
        title = "Implement SQLite backup and disaster recovery procedures"
        body = "Provide backup/restore scripts, verification tests, and operational runbooks."
        milestone = "M6 Documentation"
        labels = "area/database,type/feature,priority/high,phase/mvp"
    },
    @{
        title = "Author comprehensive documentation tree and ADRs"
        body = "Write full documentation under docs/ covering architecture, installation, SQLite, and 5 ADRs."
        milestone = "M6 Documentation"
        labels = "area/docs,type/docs,priority/high,phase/mvp"
    },

    # Future Phases
    @{
        title = "Design PostgreSQL scale adapter"
        body = "Prepare abstract database repository contracts for future high-concurrency PostgreSQL deployment."
        milestone = "M7 PostgreSQL Adapter"
        labels = "area/database,type/feature,priority/low,phase/postgres"
    },
    @{
        title = "Plan webhooks, Compose, rollback, and multi-node clustering"
        body = "Roadmap advanced features for future platform expansion."
        milestone = "M8 Advanced Platform"
        labels = "area/backend,type/feature,priority/low,phase/future"
    }
)

Write-Host "Creating initial backlog on $Repo..." -ForegroundColor Cyan

$mapping = @()

foreach ($item in $issues) {
    Write-Host "Creating: $($item.title)"
    $out = gh issue create --repo "$Repo" --title "$($item.title)" --body "$($item.body)" --milestone "$($item.milestone)" --label "$($item.labels)"
    if ($out -match '/issues/(\d+)') {
        $num = $Matches[1]
        Write-Host "Created issue #$num" -ForegroundColor Green
        $mapping += [PSCustomObject]@{
            Number = $num
            Title = $item.title
            Milestone = $item.milestone
        }
    }
}

$mapping | Export-Csv -Path "scripts/issues_created.csv" -NoTypeInformation
Write-Host "All issues created and recorded in scripts/issues_created.csv" -ForegroundColor Green
