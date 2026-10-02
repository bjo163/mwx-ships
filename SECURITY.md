# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x (MVP) | :white_check_mark: |

## Reporting a Vulnerability

We take the security of Moonships very seriously. If you discover a vulnerability, please report it via private coordinated disclosure.

**DO NOT disclose vulnerabilities in public issues or discussions.**

### Reporting Process
1. Navigate to our [Security Advisories Draft page](https://github.com/bjo163/mwx-ships/security/advisories/new).
2. Detail the exact nature of the vulnerability, including:
   - Vulnerability category (e.g. Command Injection, SSRF, Secret Exposure, Path Traversal)
   - Step-by-step reproduction steps
   - Potential impact
   - Suggested mitigations or patches if available
3. The Moonships core team will acknowledge receipt within 48 hours and coordinate release of a security patch.

## Security Architecture Highlights

- **Zero Plaintext Secrets**: All SSH private keys and sensitive environment variables are encrypted at rest using AES-256-GCM authenticated encryption.
- **Strict Parameter Passing**: Docker CLI and SSH remote executions avoid shell interpolation, employing structured arguments and input sanitization.
- **Path Sanitization**: All repository paths, workspace directories, and Docker contexts are strictly validated against directory traversal attacks.
