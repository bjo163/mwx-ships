# ADR 0004: Docker Container Runtime & Traefik Reverse Proxy

## Status
Accepted

## Context
Deploying applications across diverse operating systems and runtimes (Node.js, Rust, Python, Go, PHP) requires a standard containerization runtime. Building custom container sandboxes or requiring Kubernetes introduces immense operational complexity.

In addition, dynamic HTTP/HTTPS ingress routing with automated Let's Encrypt SSL certificates is needed for web applications.

## Decision
We select standard **Docker Engine** as the execution runtime and **Traefik** as the canonical reverse proxy:
1. **Docker Engine**: Applications are packaged as standard Docker images (built from a Git repository `Dockerfile` or pulled from a container registry).
2. **Dynamic Traefik Labels**: Rather than rewriting Nginx configuration files or implementing a custom reverse proxy, Moonships labels running containers with Traefik router and service rules (e.g. `traefik.http.routers.app.rule=Host(...)`).
3. **Structured Execution**: All Docker interactions are performed using structured arguments with strict input validation to prevent shell injection vulnerabilities.

## Consequences
- Traefik automatically discovers containers, routes traffic to internal ports, and manages ACME SSL certificates without restarting.
- Any server capable of running Docker Engine can serve as a target host.
- Custom reverse proxies are strictly avoided.
