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
    docker compose -f examples/docker-compose.yml up -d

# Stop example PostgreSQL
examples-down:
    docker compose -f examples/docker-compose.yml down -v

# Parse all example .rosql files (shows compiled SQL for each query)
run-examples:
    #!/usr/bin/env bash
    set -e
    cargo build --features server --bin rosql-parser 2>/dev/null
    for file in examples/queries/*.rosql; do
        echo "=== $(basename $file) ==="
        # Read each non-empty, non-comment line as a separate query
        while IFS= read -r line; do
            line=$(echo "$line" | sed 's/^[[:space:]]*//')
            [[ -z "$line" || "$line" == --* ]] && continue
            echo "  > $line"
            ./target/debug/rosql-parser parse "$line" 2>/dev/null | head -5
            echo ""
        done < "$file"
        echo ""
    done

# Run all checks (build + test + clippy + fmt + buf-lint)
check: build test clippy fmt buf-lint
