# Git Service Integration

The `GitService` handles repository synchronization without coupling Moonships to any single Git provider (GitHub, GitLab, Gitea, or self-hosted Git are all natively supported).

## Supported Protocols

1. **HTTPS**:
   - Format: `https://github.com/org/repo.git`
   - Public repositories or authenticated via personal access tokens in URL.
2. **SSH**:
   - Format: `git@github.com:org/repo.git` or `ssh://git@git.internal.net:2222/org/repo.git`
   - Cloned using the server's registered SSH key.

## Security Controls

- **URL Validation**: Rejects URLs containing shell metacharacters (`;`, `&`, `|`, `` ` ``, `$`).
- **Branch Validation**: Rejects invalid branch names, leading flags (e.g. `--upload-pack`), and path traversal segments (`..`).
- **Commit Inspection**: Retrieves commit SHA (`git rev-parse HEAD`) and commit message (`git log -1 --pretty=%B`) for persistent deployment audit logs.
