.PHONY: all build check test clippy fmt migrate run clean docker-build backup restore

all: check test

build:
	cargo build

check:
	cargo check --all-targets --all-features

test:
	cargo test --all-targets --all-features

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all -- --check

fmt-fix:
	cargo fmt --all

migrate:
	cargo run --bin moonships-cli -- db migrate

run:
	cargo run --bin moonships-cli -- start --server-and-worker

docker-build:
	docker build -t moonships:latest .

backup:
	@powershell -ExecutionPolicy Bypass -File .\scripts\backup-sqlite.ps1

restore:
	@powershell -ExecutionPolicy Bypass -File .\scripts\restore-sqlite.ps1

clean:
	cargo clean
