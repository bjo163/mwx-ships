# ADR 0001: Independent Deployment Control Plane Architecture

## Status
Accepted

## Context
Self-hosted application deployments traditionally require operators to manage heavy orchestration frameworks like Kubernetes, nomad, or complex cloud-specific control planes. Alternatively, monolithic multi-tenant SaaS platforms store infrastructure credentials on third-party servers.

There is a demand for a streamlined, self-hosted deployment control plane—inspired conceptually by tools like Coolify—that gives developers complete control over their physical or virtual servers without external dependencies.

## Decision
We architect **Moonships** as a dedicated, independent deployment control plane:
1. **Control Plane, Not Runtime Engine**: Moonships does not execute user applications directly within its own process space. Instead, it instructs standard Docker engines on target Linux machines over secure SSH transport.
2. **Thin Controllers & Clean Services**: REST controllers strictly parse input, authorize requests, and delegate to domain services (`GitService`, `SshService`, `DockerService`, `ProxyService`).
3. **Asynchronous Orchestration**: Deployment requests return `202 Accepted` immediately, enqueuing a durable job into a persistent worker queue (`DeploymentWorker`).
4. **No Kubernetes Dependency**: We target single-node and multi-node Docker hosts using Traefik dynamic labels for HTTP/HTTPS routing.

## Consequences
- Operators can run Moonships on low-cost hardware (e.g. 1GB VPS or local Raspberry Pi) while controlling multiple remote servers.
- The separation between control plane and container runtime prevents control plane crashes from affecting running production applications.
- Traefik integration provides zero-configuration SSL/TLS routing via standard Docker container labels.
