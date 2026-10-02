# Proxy Service & Traefik Routing

Moonships integrates with **Traefik** as its canonical reverse proxy to provide automated ingress routing and SSL/TLS certificates.

## Why Traefik?

- **Zero Reloads**: Traefik reads container configurations dynamically via the Docker socket without restarting the proxy.
- **Automated Let's Encrypt**: Manages ACME challenges and automatic certificate renewal.
- **No Custom Reverse Proxy**: Moonships avoids maintaining a fragile custom proxy daemon.

## Dynamic Label Generation

The `ProxyService` generates Traefik dynamic labels attached directly to running application containers:

```text
traefik.enable=true
traefik.http.routers.moonships-<slug>.rule=Host(`app.example.com`) || Host(`api.example.com`)
traefik.http.routers.moonships-<slug>.entrypoints=websecure
traefik.http.routers.moonships-<slug>.tls=true
traefik.http.routers.moonships-<slug>.tls.certresolver=letsencrypt
traefik.http.services.moonships-<slug>.loadbalancer.server.port=8080
```

## Domain Validation

Hostnames added to applications (`POST /api/applications/:id/domains`) must conform to **RFC 1123**:
- Max 253 characters total length.
- Labels between 1 and 63 alphanumeric characters and hyphens.
- No leading or trailing hyphens in domain segments.
