# Deployment State Machine & Lifecycle

Every deployment in Moonships passes through a deterministic finite state machine.

```
       [ queued ]
           │
           ▼
      [ cloning ] ──────────┐
           │                │
           ▼                │
      [ building ] ─────────┤
           │                │
           ▼                │  Failure at any stage
    [ stopping_old ] ───────┼──────────────────────► [ failed ]
           │                │
           ▼                │
    [ starting_new ] ───────┤
           │                │
           ▼                │
   [ healthchecking ] ──────┘
           │
           ▼
       [ success ]
```

## State Definitions

| State | Description |
|---|---|
| `queued` | Deployment record created and enqueued in SQLite persistent queue. |
| `cloning` | Git repository is being cloned or fetched, and the target branch/commit checked out. |
| `building` | Docker image is being built via `docker build` or pulled from a registry. |
| `stopping_old` | Existing application container is gracefully stopped and removed. |
| `starting_new` | New container starts with mapped ports, environment variables, and Traefik labels. |
| `healthchecking` | HTTP GET requests test the configured healthcheck path until 200 OK or timeout. |
| `success` | Container is verified healthy; application status transitions to `running`. |
| `failed` | An error occurred; exit code and error message are persisted; logs capture the failure. |
| `cancelled` | Deployment was manually cancelled by operator before execution began. |

## Failure Handling

- If any step returns a non-zero exit code or fails a validation check:
  1. The deployment state immediately updates to `failed`.
  2. The exact error reason is recorded in `error_code` and `error_message`.
  3. System and stderr streams capture the diagnostic output.
  4. The application status updates to `failed` (or remains in its previous state if no container was stopped).
