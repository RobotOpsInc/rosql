# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.5] - 2026-03-25

### Changed

- README: trademark notice, pronunciation guide, full name expansion, rosql.org link, Robot Ops attribution
- README: restructured Quick Start — showcase query with realistic output first, then CLI, then library
- README: `cargo add rosql` replaces hardcoded version in library snippet

## [0.1.4] - 2026-03-25

### Fixed

- crates.io: capture auth action output token and pass via CARGO_REGISTRY_TOKEN

## [0.1.3] - 2026-03-25

### Fixed

- crates.io: use `rust-lang/crates-io-auth-action` for OIDC trusted publishing (no API token needed)

## [0.1.2] - 2026-03-25

### Fixed

- Release workflow: use `--notes-file` for CHANGELOG extraction (fixes shell interpretation of backticks)
- Release workflow: `--allow-dirty` for cargo publish (Cargo.lock changes during build)
- npm: initial `@robotops/rosql` package creation for OIDC trusted publishing

## [0.1.1] - 2026-03-25

### Added

- **CI/CD**: Enhanced CI with parallel jobs (lint, test, test-postgres, buf-lint, cargo-deny)
- **Release workflow**: Manual-trigger workflow for GitHub Releases, Linux binaries (x64 + arm64), crates.io and npm publishing
- **npm OIDC**: Trusted publishing via GitHub OIDC (no NPM_TOKEN needed)
- **CHANGELOG**: Keep-a-changelog format
- **cargo-deny**: License, bans, and sources auditing

### Changed

- npm package scoped as `@robotops/rosql`
- Added `cdylib` crate-type for WASM compilation
- Fixture SQL files numbered for deterministic load order

## [0.1.0] - 2026-03-24

### Added

- **Parser**: Full ROSQL lexer (logos-based), recursive descent parser, typed AST with serde support
- **Unit system**: 13 categories (~65 units) with SI normalisation, affine conversion for temperature, Haversine for geographic coordinates
- **SQL compiler**: AST-to-SQL compilation for PostgreSQL and MySQL dialects
- **Schema profiles**: OtelPostgres (lowercase) and OtelClickhouse (PascalCase) column naming conventions
- **Drivers**: Native PostgreSQL and MySQL drivers via sqlx (PgPool, MySqlPool)
- **Proto schema**: parser_service.proto, ast.proto, result.proto, field_registry.proto with prost-build integration
- **CLI**: `rosql` binary with parse, compile, query, validate, completions, and serve subcommands
- **gRPC server**: ROSQLParserService implementation (Parse, Validate, GetCompletions) on Unix socket
- **WASM**: wasm-bindgen exports (parse, validate, get_completions) for frontend editors
- **Completions engine**: Context-aware autocomplete for data sources, fields, units, keywords
- **AST conversion**: Native Rust AST to proto conversion layer
- **Compound clauses**: MESSAGE JOURNEY, MESSAGE PATHS, DURING, HEALTH, ANOMALY, PATH DEVIATION, CORRELATE, SHOW RECORDING, TRACE
- **Examples**: PostgreSQL Docker Compose setup with Nav2 fixture data (3 navigation actions), example .rosql query files
- **Error handling**: Parse errors with line/column and "did you mean?" suggestions, mutation rejection, reserved syntax errors, data source availability errors
