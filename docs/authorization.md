# Authorization Model

## MVP Scope: Single Admin / Operator Model

For the initial MVP, Moonships adopts a single-operator / admin model:
- Any verified authenticated user possesses administrator privileges over registered servers, projects, applications, and deployments.
- This deliberately avoids introducing multi-tenant RBAC complexity into the MVP while keeping the security surface minimal and verifiable.

## Future Authorization Evolution (Milestone M8)

The future scale path introduces:
- **Teams & Workspaces**: Logical grouping of projects and resources.
- **Role-Based Access Control (RBAC)**:
  - `Admin`: Full cluster, server, secret, and deployment rights.
  - `Developer`: Deploy, restart, view logs, edit non-secret environment variables.
  - `Viewer`: Read-only access to metrics, logs, and application status.
- **Audit Logging**: Structured user action tracking.
