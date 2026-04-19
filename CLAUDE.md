# CLAUDE.md — ROSQL

## Project

ROSQL is a Rust parser/compiler that translates a SQL-like DSL for ROS2 robotics telemetry into backend-specific SQL (PostgreSQL, MySQL, DuckDB/Parquet). It ships as a Rust library, CLI binary, gRPC server, and WASM package. The open-source core (parser, AST, compiler, drivers) is at `src/`. The documentation site is at `website/`.

## Design Philosophy

ROSQL is designed for **humans** — specifically robotics engineers who know SQL but not necessarily database internals. Keep this in mind on every change:

- **Syntax should read naturally.** A robotics engineer should be able to guess `FROM traces WHERE status = 'ERROR' SINCE 1 hour ago` without reading docs.
- **Clarity over brevity.** Prefer `PATH DEVIATION FOR ROBOT 'r1'` over terse abbreviations.
- **Every user-facing surface is a design surface.** CLI flags, error messages, docs, and example queries are all part of the language's UX.
- **Documentation should be intuitive and elegantly communicated.** A user should never encounter a discrepancy between what the docs say and what the code does.

## Before Every Push

Always run these before committing Rust changes — CI enforces both and failures are the most common cause of red builds:

```sh
cargo fmt            # auto-formats in place; never use --check here, just fix it
cargo clippy -- -D warnings   # must be clean; fix warnings, don't suppress them
```

Or run both via:

```sh
just check           # build + test + clippy + fmt --check + buf-lint
```

`cargo fmt` is the most frequent CI failure. It reformats long `assert!` messages, chained method calls, and multi-line strings in ways that aren't obvious. Always run it after writing tests.

## Quick Reference

```sh
just check           # build + test + clippy + fmt + buf-lint (run before every push)
just test            # cargo test (unit + snapshot + compile tests)
just test-duckdb     # cargo test --features duckdb (Parquet integration, no Docker)
just test-examples   # Postgres integration tests (spins up Docker)
just test-wasm       # wasm-pack test --headless --chrome --features wasm

cargo build --features server --bin rosql   # CLI / gRPC binary

cd website && pnpm install && pnpm start    # Docusaurus dev server
```

## Architecture: The Integrity Chain

```
ROSQL query string  →  parser.rs  →  AST (ast.rs)
                    →  compiler.rs + SqlDialect  →  dialect SQL string
                    →  sql.rs (ROSQLBackend trait)  →  sqlx / embedded DuckDB
                    →  ROSQLResult  (universal output format)
```

**Key invariant:** The input (ROSQL query string) and output (`ROSQLResult`) are backend-agnostic. The SQL dialect is an internal detail.

Key source files:
- `src/parser.rs` — lexer + recursive descent parser
- `src/ast.rs` — AST types (Query, ROSQLQuery, PipelineQuery, CompoundQuery)
- `src/drivers/compiler.rs` — ONE compiler with dialect branches, not separate compilers
- `src/drivers/dialect.rs` — `SqlDialect::PostgreSQL | MySQL | DuckDB`
- `src/drivers/sql.rs` — `ROSQLBackend` trait + `SqlBackend` impl (sqlx pool or DuckDB conn)
- `src/drivers/otel_registry.rs` — default OpenTelemetry field registry, canonical metric names
- `src/drivers/field_registry.rs` — `FieldRegistry` and schema profiles

**Schema profiles:** `OtelPostgres` uses snake_case columns (`trace_id`, `span_name`); `OtelClickhouse` uses PascalCase (`TraceId`, `SpanName`). Set via `--schema` flag.

**`--backend parquet` vs `--features duckdb`:** The CLI flag is `parquet`; the Cargo feature flag is still `duckdb`. These are different things — do not confuse them.

## Driver Conformance

Every ROSQL query must produce **semantically equivalent results** across all supported backends. When adding or modifying a clause:

1. Add parser support + insta snapshot tests
2. Add compile branches for **all three dialects** in `compiler.rs`
3. Add `compile_sql()` assertions for **all supported dialects** (currently PostgreSQL and DuckDB; extend when new drivers land)
4. **MySQL has zero compile test coverage** — add tests when touching MySQL codepaths
5. If the feature touches execution (not just compilation), add integration test coverage

Integration tests:
- **Postgres:** `tests/postgres_integration.rs`, `#[ignore]`, requires Docker → `just test-examples`
- **Parquet:** `tests/parquet_integration.rs`, embedded DuckDB → `just test-duckdb`

## Testing Patterns

**Parser tests** (`tests/parse_*.rs`) — insta YAML snapshots in `tests/snapshots/`
- After changing parser output: `cargo insta review` to accept/reject diffs
- Snapshot names follow: `{test_module}__{test_name}.snap`
- New snapshots fail on first run; accept them with `cargo insta review` or by renaming `.snap.new` → `.snap`

**Compile tests** (`tests/compile_audit.rs`) — assert on generated SQL strings
- `compile_sql(query, dialect)` → returns SQL string, panics on parse/compile error
- `compile_err(query, dialect)` → returns `ROSQLError`, panics if compile succeeds
- Test both `SqlDialect::PostgreSQL` and `SqlDialect::DuckDB` for every new clause

**Roundtrip / property tests** — `tests/roundtrip.rs`, `tests/property.rs`

**Doc-query smoke tests** (`tests/doc_queries.rs`) — parse + compile every query in `examples/queries/doc_examples.rosql` against both PostgreSQL and DuckDB. Run with `cargo test --test doc_queries`.

## Testing Integrity

These rules are enforced by CI and must not be bypassed:

1. **Doc-query rule** — Every ROSQL query added to any documentation surface must have a corresponding entry in `examples/queries/doc_examples.rosql`. If a query is removed from docs, remove it here too. See the corpus file header for the gated-clause allowlist (HEALTH, CORRELATE, SHOW RECORDING).

2. **All-dialects rule** — Every new compile test in `tests/compile_audit.rs` must assert on all supported dialects. Today that means `SqlDialect::PostgreSQL` and `SqlDialect::DuckDB`; when a new driver lands, existing and new tests must be extended to cover it. Use the `assert_compiles_both` helper (or a future `assert_compiles_all` variant). Single-dialect-only tests are not acceptable for new code.

3. **Combination test rule** — When adding a new clause or modifier, add at least one compile test combining it with 2+ other clauses (e.g. new clause + WHERE + SINCE). This catches silent-drop bugs like the DURING combined-form issue.

4. **CLI output rule** — If the CLI output format changes (JSON fields, exit codes), update `tests/cli_integration.rs`.

## Documentation Sync Checklist

**Every feature addition or syntax change must update ALL applicable surfaces.** Drift between code and docs is the most common recurring mistake in this project.

| # | Surface | Path |
|---|---------|------|
| 1 | README | `README.md` |
| 2 | Changelog | `CHANGELOG.md` |
| 3 | Docs (next) | `website/docs/` |
| 4 | Docs (current stable) | `website/versioned_docs/version-0.5/` |
| 5 | Homepage | `website/src/pages/index.tsx` |
| 6 | FAQ | `website/src/pages/faq.tsx` |
| 7 | Examples page | `website/src/pages/examples.tsx` |
| 8 | REPL example queries | `website/src/components/RosqlRepl.tsx` |
| 9 | Playground | `website/src/pages/playground.tsx` |
| 10 | Example queries | `examples/queries/` |
| 11 | Test coverage | `examples/queries/doc_examples.rosql` + `tests/doc_queries.rs` |

When **removing** a syntax form, grep all of the above for the old name — removed features reliably linger in 7+ surfaces.

Fixtures that must stay schema-consistent:
- SQL: `website/static/fixtures/`
- Parquet: `examples/parquet/fixtures/`

## Versioned Docs Rules

- `website/docs/` — next (unreleased) version
- `website/versioned_docs/version-0.5/` — current stable
- `website/versioned_docs/version-0.4/` — previous version
- `website/versioned_docs/version-0.3/` — legacy (different syntax)

Rules:
- **Never backport v0.5+ features into v0.4 or v0.3 docs.** v0.3 has different syntax and conventions (e.g. `MESSAGE JOURNEY` is valid in v0.3 but removed in v0.4.1).
- **Patch releases:** edit `versioned_docs/version-X.Y/` in place. Do not cut a new snapshot.
- **Minor version bumps only:** `cd website && pnpm docusaurus docs:version X.Y`

## Release Workflow

1. Branch from `development`
2. Make changes; run `just check` — all checks must pass
3. Add a CHANGELOG entry: `## [x.y.z] - YYYY-MM-DD` with `### Added` / `### Changed` / `### Removed`
4. Bump the version in `Cargo.toml` (see CONTRIBUTING.md: `just bump-version [major|minor|patch]`)
5. PR against `development`, then `development` → `main`
6. Merge to `main` triggers automated: git tag, GitHub Release, crates.io publish, npm publish
7. Docs auto-deploy when `website/**` changes merge to `main`

## Common Pitfalls

- **Docs-code drift** — features removed from code reliably linger in 7+ surfaces. Always grep all surfaces in the checklist above before and after a change.
- **`--backend parquet` vs `--features duckdb`** — different things. The CLI flag is `parquet`; the Cargo feature is `duckdb`.
- **DURING clause** — standalone `DURING(FROM source WHERE ...)` is implemented. The combined form `FROM traces WHERE ... DURING(...)` is **not yet wired** in the standard-query parser (silently dropped). See GH#74.
- **Resource attribute keys** — use dotted notation (`robot.id`, `host.name`), not snake_case (`robot_id`).
- **Docusaurus CSS** — `<pre>` elements have default backgrounds from Docusaurus's code block styles that win over parent div backgrounds. Use inline `style={{ background: 'transparent' }}` on `<pre>` when overriding.
- **Proto codegen** — `src/proto/rosql.v1.rs` is auto-generated by `build.rs` on every `cargo build`. It is gitignored — never commit it.
- **MySQL coverage** — MySQL has zero compile test coverage. Note this in any PR that adds MySQL-path changes.
