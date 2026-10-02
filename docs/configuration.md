# Moonships Configuration Reference

Moonships configurations are managed through YAML environment files located in `config/` along with environment variables.

## Configuration Files Hierarchy

- `config/development.yaml`: Local development settings.
- `config/test.yaml`: Disposable test database and mock mailer.
- `config/production.yaml`: Production-hardened defaults.

## Key Configuration Directives

### 1. Database & SQLite
```yaml
database:
  uri: <%= get_env(name="DATABASE_URL", default="sqlite://data/moonships.sqlite?mode=rwc") %>
  enable_logging: false
  min_connections: 1
  max_connections: 5
  auto_migrate: true
```

### 2. Persistent SQLite Queue
```yaml
workers:
  mode: BackgroundQueue

queue:
  kind: Sqlite
  uri: <%= get_env(name="QUEUE_URL", default="sqlite://data/moonships.sqlite?mode=rwc") %>
  dangerously_flush: false
```

### 3. Web Server & Frontend Static Assets
```yaml
server:
  port: <%= get_env(name="PORT", default="5150") %>
  host: http://localhost
  middlewares:
    static:
      enable: true
      must_exist: true
      folder:
        uri: "/"
        path: "frontend/dist"
      fallback: "frontend/dist/index.html"
```

### 4. JWT Authentication
```yaml
auth:
  jwt:
    secret: <%= get_env(name="JWT_SECRET", default="replace_with_strong_secret") %>
    expiration: 604800 # 7 days in seconds
```

### 5. Critical Environment Variables

| Variable | Description | Default |
|---|---|---|
| `ENCRYPTION_KEY` | 32-byte hex key for AES-256-GCM secret encryption | Required in production |
| `DATABASE_URL` | SQLite database URI | `sqlite://data/moonships.sqlite?mode=rwc` |
| `PORT` | HTTP port for REST API and web UI | `5150` |
| `JWT_SECRET` | Secret key for signing JWT tokens | Configured in config |
