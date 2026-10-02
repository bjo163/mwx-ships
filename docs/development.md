# Moonships Development Guide

## Local Development Workflow

### 1. Requirements
- Rust stable toolchain
- Node.js 20+ and npm
- SQLite 3 CLI (optional, for inspecting `data/moonships.sqlite`)

### 2. Environment Setup
```bash
cp .env.example .env
mkdir -p data
```

### 3. Running Backend in Dev Mode
```bash
# Run database migrations
cargo loco db migrate

# Start backend server and worker
cargo loco start --server-and-worker
```

### 4. Running Frontend Live Dev Server
```bash
cd frontend
npm install
npm run dev
```
Vite will proxy API requests to `http://localhost:5150`.

### 5. Running Quality Checks
```bash
# Code format check
cargo fmt --check

# Type & macro checks
cargo check -j 2

# Linting with strict warnings
cargo clippy --all-targets --all-features -- -D warnings

# Run test suite
cargo test -j 2
```
