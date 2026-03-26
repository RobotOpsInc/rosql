# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.2] - 2026-03-26

### Added

- `NOTICE` file (Apache 2.0 copyright attribution for Robot Ops Inc.)
- `about.toml` and `about.hbs` for `cargo-about` third-party license attribution
- `just notices` recipe to generate `THIRD_PARTY_NOTICES`
- `just check-licenses` recipe to verify all dependency licenses are approved
- CI `license-check` job: validates attribution can be generated on every PR
- Release binaries now include `NOTICE`, `LICENSE`, and `THIRD_PARTY_NOTICES` in tarballs

## [0.3.1] - 2026-03-26

### Fixed

- CI: build WASM package before website build in deploy workflow
- `cargo fmt`: line-length reformatting in `dialect.rs`, `sql.rs`, `wasm.rs`
- Homepage OG image replaced with actual homepage screenshot
- Homepage unified output flow diagram section added between REPL and Why ROSQL?

## [0.3.0] - 2026-03-25

### Added

- DuckDB embedded driver (`duckdb` feature flag) — no Docker or external server required
- `SqlDialect::DuckDB` with correct `NOW()::TIMESTAMP` cast for interval arithmetic
- DuckDB CLI support: `--backend duckdb --url duckdb:///path/to/db`
- DuckDB integration tests (10 tests, no Docker needed)
- `examples/duckdb/` with fixtures and setup guide
- `test-duckdb` and `test-all` justfile recipes

### Fixed

- `FACET` compiler: `SELECT * GROUP BY col` now generates `SELECT col, COUNT(*) AS count GROUP BY col`
- WASM REPL `compile()` now uses `SqlDialect::DuckDB` for correct interval SQL
- REPL BigInt serialization error for `HEALTH()` and `ANOMALY()` results

## [0.2.0] - 2026-03-25

### Added

- rosql.org documentation and marketing website (Docusaurus, GitHub Pages)
  - Versioned docs: quickstart, drivers, CLI, WASM, cookbook, schema reference, command reference
  - Home page with live WASM REPL widget (CodeMirror + `@robotops/rosql`)
  - Examples, FAQ, benchmarks, playground, and contributing pages
  - Robot Ops design system: Space Grotesk, JetBrains Mono, brand red `#E74C3C`
  - PostHog analytics, sitemap, JSON-LD structured data, robots.txt
  - GitHub Actions deploy workflow on push to `main`

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
