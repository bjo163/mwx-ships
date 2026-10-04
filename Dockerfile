# ==========================================
# MOONSHIPS MULTI-STAGE DOCKERFILE
# ==========================================

# 1. Frontend Build Stage
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# 2. Rust Build Stage
FROM rust:1.94-slim-bookworm AS backend-builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    sqlite3 \
    libsqlite3-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY migration/ migration/
COPY src/ src/
COPY config/ config/

# Build release binary
RUN cargo build --release --bin moonships-cli

# 3. Minimal Production Runtime Stage
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    sqlite3 \
    libsqlite3-0 \
    curl \
    git \
    openssh-client \
    && rm -rf /var/lib/apt/lists/*

# Create persistent data directory
RUN mkdir -p /app/data && chown -R 1000:1000 /app

# Copy binary from builder
COPY --from=backend-builder /app/target/release/moonships-cli /usr/local/bin/moonships-cli
# Copy config
COPY --from=backend-builder /app/config /app/config
# Copy frontend assets
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

ENV LOCO_ENV=production
ENV PORT=5150
ENV BINDING=0.0.0.0
ENV DATABASE_URL=sqlite:///app/data/moonships.sqlite?mode=rwc
ENV QUEUE_URL=sqlite:///app/data/moonships.sqlite?mode=rwc

EXPOSE 5150

VOLUME ["/app/data"]

# v0.2 executes application workloads on selected remote servers via SSH.
# The control plane therefore does not need root or access to a Docker socket.
USER 1000:1000

CMD ["moonships-cli", "start", "--server-and-worker"]
