# Moonships Installation Guide

Moonships runs as a single binary with an embedded SQLite database. No external database or queue infrastructure is required.

## Prerequisites

- **Rust**: 1.80+ (stable toolchain)
- **Node.js**: 20+ (for frontend compilation)
- **Docker**: For running Moonships in a container or managing target machines

## Method 1: Docker Compose (Recommended for Production)

1. Clone repository:
   ```bash
   git clone https://github.com/bjo163/mwx-ships.git
   cd mwx-ships
   ```
2. Configure environment:
   ```bash
   cp .env.example .env
   # Generate a 32-byte hex key for ENCRYPTION_KEY
   ```
3. Start Moonships:
   ```bash
   docker compose up -d
   ```
4. Access dashboard at `http://localhost:5150`.

## Method 2: Native Binary Installation

1. Clone and enter directory:
   ```bash
   git clone https://github.com/bjo163/mwx-ships.git
   cd mwx-ships
   ```
2. Build frontend assets:
   ```bash
   cd frontend
   npm install
   npm run build
   cd ..
   ```
3. Run database migrations:
   ```bash
   cargo loco db migrate
   ```
4. Launch Moonships server and persistent worker:
   ```bash
   cargo loco start --server-and-worker
   ```
