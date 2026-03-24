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

# Run all checks (build + test + clippy + fmt)
check: build test clippy fmt
