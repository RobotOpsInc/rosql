# ROSQL development commands

# Build with default features
build:
    cargo build

# Run all tests
test:
    cargo test

# Build WASM package
build-wasm:
    cargo build --target wasm32-unknown-unknown --features wasm

# Run clippy lints
clippy:
    cargo clippy -- -D warnings

# Check formatting
fmt:
    cargo fmt --check

# Format code
fmt-fix:
    cargo fmt

# Lint proto files
buf-lint:
    buf lint proto/

# Regenerate proto types (build.rs handles this automatically on cargo build)
generate-proto:
    cargo build

# Start example PostgreSQL with fixture data
examples-up:
    docker compose -f examples/postgres/docker-compose.yml up -d

# Stop example PostgreSQL
examples-down:
    docker compose -f examples/postgres/docker-compose.yml down -v

# Run example integration tests against Docker PostgreSQL
test-examples:
    #!/usr/bin/env bash
    set -e
    echo "Starting PostgreSQL..."
    docker compose -f examples/postgres/docker-compose.yml up -d --wait
    echo "Running integration tests..."
    DATABASE_URL=postgresql://rosql:rosql@localhost:5432/rosql_examples \
        cargo test --ignored --features postgres 2>&1 || { docker compose -f examples/postgres/docker-compose.yml down -v; exit 1; }
    echo "Tearing down..."
    docker compose -f examples/postgres/docker-compose.yml down -v

# Lint (clippy + fmt)
lint: clippy fmt

# Extract version from Cargo.toml
validate-version:
    @grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'

# Run all checks (build + test + clippy + fmt + buf-lint)
check: build test clippy fmt buf-lint
