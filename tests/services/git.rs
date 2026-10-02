use moonships::services::git::GitService;

#[test]
fn test_valid_git_urls() {
    assert!(GitService::validate_url("https://github.com/bjo163/mwx-ships.git").is_ok());
    assert!(GitService::validate_url("http://gitlab.local/infra/repo.git").is_ok());
    assert!(GitService::validate_url("git@github.com:bjo163/mwx-ships.git").is_ok());
    assert!(GitService::validate_url("ssh://git@git.internal.net:2222/org/project.git").is_ok());
}

#[test]
fn test_rejects_injected_git_urls() {
    assert!(GitService::validate_url("https://github.com/evil/repo; rm -rf /").is_err());
    assert!(GitService::validate_url("https://github.com/evil/repo && curl evil.com").is_err());
    assert!(GitService::validate_url("https://github.com/evil/`reboot`").is_err());
    assert!(GitService::validate_url("file:///etc/passwd").is_err());
    assert!(GitService::validate_url("").is_err());
}

#[test]
fn test_branch_validation() {
    assert!(GitService::validate_branch("main").is_ok());
    assert!(GitService::validate_branch("feature/cool-deploy").is_ok());
    assert!(GitService::validate_branch("v1.0.0").is_ok());

    assert!(GitService::validate_branch("-bad-flag").is_err());
    assert!(GitService::validate_branch("branch with spaces").is_err());
    assert!(GitService::validate_branch("branch..traversal").is_err());
    assert!(GitService::validate_branch("").is_err());
}
