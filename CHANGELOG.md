# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.4] - 2026-05-20

### Added

- **`FROM tf` field registry entries** — `tf_states` now has bare-column field registrations (`timestamp`, `org_id`, `robot_id`, `parent_frame`, `child_frame`, `translation_{x,y,z}`, `rotation_{x,y,z,w}`). Previously the `tf` data source was wired in the parser/compiler but the columns weren't exposed to the validator, so `FROM tf WHERE parent_frame = ...` produced an unhelpful error. Compile-test coverage added across all three dialects.
- **`tf_states` fixture and Parquet view** — new `examples/duckdb/fixtures/10_tf_states.sql` with TF data for the three demo robots; `tf_states/` added to the Parquet backend view mappings and regenerated fixture set so `FROM tf` works end-to-end with `--backend parquet`.
- **`org_id` column on `tf_states`** — added to the example DDL (DuckDB + PostgreSQL fixtures and schema-reference docs) for parity with the upstream TF table schema.

## [0.5.3] - 2026-04-19

### Added

- **Comprehensive query coverage audit (issue #96)** — every ROSQL query in user-facing documentation is now smoke-tested against both PostgreSQL and DuckDB via `examples/queries/doc_examples.rosql` + `tests/doc_queries.rs`. Two doc queries that silently produced wrong results were caught and fixed.
- **All-dialect compile coverage** — every SQL-output test in `tests/compile_audit.rs` now asserts on PostgreSQL, DuckDB, and MySQL via `assert_compiles_all` / `assert_compiles_dialects`. Redundant single-dialect and `_both` sibling tests removed (-471 lines).
- **Cross-backend parity tests** — `tests/compile_parity.rs` with 15 structural equivalence checks ensuring both dialects produce the same high-level constructs (GROUP BY, ORDER BY, LIMIT, CTE presence) for representative queries.
- **Multi-clause combination tests** — 19 tests covering compound clause combinations; the DURING combined-form gap is now explicitly documented as `#[ignore]` pending GH#74.
- **MySQL baseline coverage** — 10 compile tests with MySQL-specific substring assertions (`NOW() - INTERVAL`, `JSON_EXTRACT`, `FROM_UNIXTIME`, etc.).
- **CLI integration tests** — `tests/cli_integration.rs` covering `compile`, `parse`, `validate`, and `completions` subcommands with JSON shape assertions; requires `--features server`.
- **WASM shape assertions** — `tests/wasm.rs` upgraded from liveness-only to structural assertions on `ok`/`valid`/`ast`/`errors` fields and completions array shape.
- **Testing Integrity policy in CLAUDE.md** — four enforceable rules: doc-query rule, all-dialects rule, combination test rule, CLI output rule.

### Fixed

- **Parquet fixture `robot_id` corruption** — odom rows in `05_topic_messages.sql` had a timestamp expression in the `robot_id` column instead of `'robot-amr-02'`. PATH DEVIATION FOR ROBOT returned 0 rows against the Parquet backend.
- **Missing `ros2.topic.message_rate` metrics** — TOPIC_RATE('/cmd_vel') returned null in Parquet integration tests because no message-rate data existed in the metrics fixture. Added `/cmd_vel` and `/odom` publish-rate rows.
- **Stale Parquet integration test assertions** — test IDs and counts (`trace-002`, COUNT=11, 1 recording, `robot_sim_001`) referenced a fixture schema predating the PR #103 data rewrite. Synced to current data and regenerated Parquet files.
- **`SINCE last week` in docs** — `command-reference.mdx` documented `SINCE last week` as valid syntax; the parser only accepts `SINCE last <event_type>`. Changed to `SINCE 7 days ago`.
- **`FOR VERSION` before `ANOMALY` in cookbook** — `cookbook.mdx` had `FOR VERSION 'v1.3.0' ANOMALY(...)` which is invalid; reordered to `ANOMALY(...) ... FOR VERSION 'v1.3.0'`.

## [0.5.2] - 2026-04-18

### Fixed

- **`span_name_col` → `span_name`** — renamed the `otel_traces` column to match the standard OTel Collector PostgreSQL exporter convention. Updated across schema fixtures, docs, and the website Gantt visualization.
- **`s3_key` → `file_uri` on `mcap_metadata`** — replaced the S3-only field with a generic `file_uri` that accepts any URI scheme (`s3://`, `file://`, `gs://`), enabling local and GCS-backed MCAP storage.

### Added

- **`org_id` field** — queryable on `otel_traces`, `otel_logs`, and `otel_metrics` via `resource_attributes->>'organization.id'`. The ClickHouse dialect (upcoming #98) will upgrade this to the materialized `OrgId` column.
- **`system_logs` tab completion** — `FROM system_logs` now appears in CLI autocomplete alongside other data sources.

## [0.5.1] - 2026-04-17

### Added

- **Public demo dataset** — a sample AMR telemetry dataset (traces, logs, metrics, topic messages) is now hosted at `s3://robotops-production-rosql-demo/data` and updated on every release. No credentials or data source required to run a first query.
- **Quickstart zero-friction example** — the quickstart, homepage, and all versioned docs now lead with the public demo URL so new users can query real robot telemetry immediately after install.
- **Release pipeline S3 upload** — the release workflow now syncs `examples/parquet/fixtures/` to the public demo bucket automatically on each release.

## [0.5.0] - 2026-04-17

### Fixed

- **`FROM recordings SINCE`** — `SINCE` now correctly filters by `end_time` overlap semantics on `mcap_metadata` instead of generating a broken `"timestamp"` predicate against a non-existent column. `BETWEEN` uses full interval-overlap logic (`start_time <= end AND end_time >= start`).

### Changed

- Versioned docs snapshot for v0.5 created. The `website/docs/` (next) baseline has been cleaned of v0.4.x migration notes, removed-syntax warnings, and `_(v0.4.x+)_` version markers — all documented syntax is now current and fully supported.

## [0.4.13] - 2026-04-16

### Fixed

- **Homebrew tap auto-merge** — formula bump PRs on `RobotOpsInc/homebrew-tap` now auto-merge immediately after creation instead of waiting for manual review.

## [0.4.12] - 2026-04-16

### Fixed

- **npm publish** — node 22 bundles npm 10.x which has an OIDC provenance bug with scoped packages, producing a spurious 404. Switched to `npx --yes npm@11 publish` to use a known-good npm version without the self-upgrade corruption issue.

## [0.4.11] - 2026-04-16

### Fixed

- **macOS Intel CI** — `macos-13` runner is no longer available on GitHub Actions; switched to cross-compiling `x86_64-apple-darwin` from `macos-latest` (Apple Silicon) using `rustup target add x86_64-apple-darwin`.
- **npm publish** — removed `npm install -g npm@latest` which was corrupting the npm module tree on the runner; Node 22's built-in npm supports OIDC provenance publishing without upgrading.

## [0.4.10] - 2026-04-15

### Fixed

- **`install.sh` 404** — moved `install.sh` from the repo root into `website/static/` so it is actually served at `https://rosql.org/install.sh` by the Docusaurus site.

## [0.4.9] - 2026-04-15

### Added

- **macOS Intel (x86_64) binary** — pre-built tarballs for `x86_64-apple-darwin` are now published to GitHub Releases on every release. `install.sh` and the Homebrew formula cover all four targets: Linux x86_64, Linux arm64, macOS Intel, macOS Apple Silicon.
- **Homebrew tap** — `brew install robotopsinc/tap/rosql` is now the recommended macOS install method. The release workflow automatically opens a bump PR on `RobotOpsInc/homebrew-tap` after each release.
- **`rosql query` table output** — results now render as a human-readable aligned table by default. Use `--format json` to get the previous JSON output, or `--format csv` for export. Row count and execution time are printed to stderr.
- **`rosql schema` subcommand** — connects to a backend and reports which canonical OTel data sources (`traces`, `logs`, `metrics`, `topics`, `recordings`) are available.
- **`--file <path>`** flag on `parse`, `compile`, `query`, `validate` — read the query from a `.rosql` file instead of a positional argument.
- **`--no-color`** global flag — disable ANSI color codes. Color is also auto-disabled when stdout is not a TTY.
- **`ROSQL_BACKEND`, `ROSQL_URL`, `ROSQL_SCHEMA` environment variables** — set default values for the corresponding flags.
- **`~/.config/rosql/config.toml`** — persistent per-user defaults for `backend`, `url`, and `schema`. Precedence: CLI flag > env var > config file > built-in default.

### Changed

- **`rosql query` default output format changed from JSON to table** — this is a breaking change for scripts that rely on the JSON output. Replace `rosql query ...` with `rosql query ... --format json` to restore the previous behavior.

## [0.4.8] - 2026-04-15

### Fixed

- **`WITHIN` proto serialization** — `Condition::Within` (both `GPS` and `Local` center variants) is now fully serialized to protobuf instead of silently falling back to a `IsNotNull` stub. Added `WithinCondition`, `GpsCenter`, and `LocalCenter` messages to `ast.proto` and wired them into `condition_to_proto` in `convert.rs`. Two new unit tests (`convert_within_gps`, `convert_within_local`) cover both variants. Closes #74.

## [0.4.7] - 2026-04-15

### Added

- **REPL v2: rich visualizations** — the REPL now renders 11 visualization types (Gantt, StackedLineChart, LineChart, BarChart, HorizontalBars, DirectedGraph, NodeGraph, ScalarCards, LogTable, RecordingList, DataTable) automatically based on the `format_hint` returned by the ROSQL compiler. A Visual / Raw JSON toggle lets users inspect raw results.
- **3-robot AMR fleet fixture dataset** — 8 SQL fixture files (`01_schema.sql` through `08_baseline.sql`) replace the single-robot placeholder. The dataset covers 3 robots × 3 missions each, a realistic firmware-regression failure scenario (`trace-amr02-m3` with a costmap timeout), and a historical baseline for anomaly detection.
- **9 narrative showcase queries** — the REPL now ships with 9 queries that follow a complete diagnostic investigation: trace the failure → find the root cause → check CPU blast radius → inspect node topology → compare to last week's baseline.
- **"About this dataset" docs page** (`/docs/repl-dataset`) — describes the fleet, investigation story, table inventory, and timestamp design.
- **Homepage v0.4 updates** — two new "Why ROSQL?" benefit cards (unit-aware filtering, visual format hints); output format cards updated to FormatHint-specific (Gantt traces, time-series charts, topology graphs, colored tables); TIMESERIES + FACET demo section; unit-aware sensor filtering callout; REPL subtitle and below-REPL link updated.
- **Fixture consistency tests** (`tests/fixture_consistency.rs`) — 13 tests verifying cross-table referential integrity, temporal ordering, and data requirements for each showcase query.
- **Showcase query tests** (`tests/showcase_queries.rs`) — 9 tests verifying compile + format hint for all showcase queries; 5 also execute against the DuckDB fixture dataset.
- **Format hint coverage** in `tests/compile_audit.rs` — `showcase_format_hints` test asserts correct `FormatHint` for all 9 showcase queries.
- **Y-axis labels on line charts** — `LineChartViz` and `StackedLineChartViz` now display a human-readable Y-axis label (e.g. "Lateral path deviation (m)", "Cpu Usage") rendered as a rotated HTML element for reliable cross-browser display.
- **`SvgTooltip` shared component** — hover tooltips extracted into a single reusable SVG component used by `DirectedGraphViz` and `NodeGraphViz`.

### Changed

- **`PATH DEVIATION` returns per-timestamp rows** — the outer SELECT now returns one row per odometry pose (`timestamp`, `lateral_deviation_m`, `actual_x/y`, `planned_x/y`) ordered by timestamp, giving `LineChartViz` real time-series data to plot instead of a single aggregate row.
- **`format_inference` x-axis for `PathDeviation`** — updated from the non-existent `waypoint_index` to `timestamp` to match the new projection.
- **`NodeGraphViz` unified with `DirectedGraphViz` visual language** — topics are now rendered as edge labels rather than blue rectangular intermediate nodes, matching `DirectedGraphViz`. Removes the bipartite graph model, the node/topic legend, and the inconsistent blue palette.
- **Epoch timestamp formatting** — all visualization components (`LineChartViz`, `StackedLineChartViz`, `LogTableViz`) share a single `formatEpochTick` utility that detects ms/μs/ns magnitude and formats as `HH:MM:SS`, replacing per-component ad-hoc formatting.
- **`MESSAGE FLOW` compiler** — outputs explicit `source_node`, `target_node`, `topic` columns instead of `SELECT *`, ensuring `DirectedGraphViz` always receives correctly named columns.
- **`SHOW SPAN SUMMARY` compiler** — filters out `ros2.graph.*` topology spans so only application-level spans appear in the "Slowest spans" view; `format_inference` `y_axis` corrected from `duration` to `avg_duration`.

## [0.4.6] - 2026-04-11

### Added

- **Algolia DocSearch** — full-text search now available across all docs on rosql.org
- **Syntax page** — new documentation page covering ROSQL grammar, query forms, clause composition, operators, time expressions, physical units, and case sensitivity rules
- **DURING test coverage** — parse snapshot tests and compile tests for the `DURING(FROM ...)` standalone compound form; parser gap for `FROM ... DURING(...)` documented and tracked in GH#74
- **CLAUDE.md** — project conventions file encoding the integrity chain, documentation sync checklist, driver conformance rules, and common pitfalls for AI-assisted development

### Changed

- **Quickstart** — install section redesigned as three tabs (Pre-built binary · cargo install · Build from source); duplicate "Building from source" section removed
- **REPL** — removed secondary Validate button; fixed light-mode output contrast (dark terminal background now consistent across themes)
- **Docs aligned with v0.4 code** — corrected all stale examples and references accumulated across v0.4.0–v0.4.5: `MESSAGE JOURNEY` → `TRACE 'id'` in README, homepage, FAQ, examples page, REPL, and quickstart; metric field names updated to canonical OTel names (`rx_rate_hz` → `message_rate`, etc.); resource attribute key corrected to `robot.id`
- **command-reference** (v0.4) — added SHOW TOPICS, SHOW NODES, SHOW NODE GRAPH, TIMESERIES, ENRICH WITH, FORMAT, BETWEEN, STDDEV; added `system_logs`, `diagnostics`, `tf`, `heartbeats`, `events` data sources
- **schema-reference** (v0.4 + docs) — complete redesign with backend-specific tabs (PostgreSQL · ClickHouse · Parquet) for all DDL sections; added ClickHouse PascalCase DDL (`DateTime64(9)`, `Map(String, String)`, `MergeTree()`); added Parquet directory layout, glob patterns, and required column tables for each logical table; added two-tab field mapping sections (otel-postgres vs otel-clickhouse) showing exact column name differences; removed standalone Schema profiles section (now embedded in tabs); added full 29-entry metric field alias table from `otel_registry`; added `tf_states`, `robot_heartbeats`, `system_logs`, `ros2_events` table schemas
- **wasm.mdx** (v0.4) — corrected TypeScript return types for `parse()` and `validate()`; added `compile()` API documentation
- **cli.mdx** (v0.4) — added `parquet` to `--backend` options; removed undocumented gRPC `Compile` method

### Removed

- **Orphaned test snapshots** — deleted `parse_compound__snapshot_message_journey`, `__snapshot_message_paths`, and `__snapshot_message_path_from_to` (syntax removed in v0.4.1)

## [0.4.5] - 2026-04-10

### Added

- **`--backend parquet`** — new CLI backend that queries Parquet telemetry files (local path or `s3://` URI) using an embedded in-memory DuckDB instance. Replaces the old `--backend duckdb --url duckdb://...` interface.
- **`SqlBackend::from_parquet(url)`** — new public constructor that opens an in-memory DuckDB instance, creates views over `read_parquet()` globs in the expected directory layout (`traces/`, `logs/`, `metrics/`, `topic_messages/`, `mcap_metadata/`), and runs capability probing.
- **S3 support** — when `--url s3://...` is used, the DuckDB `httpfs` extension is loaded automatically and credentials are read from standard AWS environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `AWS_ENDPOINT_URL`, `AWS_PROFILE`).
- **Parquet fixture files** — pre-built Parquet fixtures in `examples/parquet/fixtures/` (generated from the SQL fixtures in `examples/duckdb/fixtures/`) for local development and integration testing.
- **Parquet integration tests** — `tests/parquet_integration.rs` replaces `tests/duckdb_integration.rs` with 22 tests covering all query types, capability errors, and Parquet-specific scenarios (missing subdirectories, nonexistent paths, S3 URL detection).
- **Install script** — `install.sh` provides a one-liner setup (`curl -fsSL https://rosql.org/install.sh | sh`) for Linux x86_64, Linux arm64, and macOS Apple Silicon.
- **macOS arm64 release builds** — pre-built binaries now include `aarch64-apple-darwin` (Apple Silicon). Release binaries now built with `--features server,duckdb` so `--backend parquet` works out of the box.

### Changed

- **`Backend::Duckdb` → `Backend::Parquet`** — the CLI enum variant for DuckDB-backed queries is now named `parquet`. The old `--backend duckdb` is no longer valid; use `--backend parquet --url <path>`.
- **`SqlDialect::from_url()` no longer recognises `duckdb://` or `md:`** — the dialect is set directly when `Backend::Parquet` is selected. Passing a `duckdb://` URL to `SqlBackend::new()` now returns a helpful error directing users to `--backend parquet`.
- **`connect()` no longer has a DuckDB branch** — DuckDB connections are created exclusively via `SqlBackend::from_parquet()`. `SqlBackend::new()` only handles `postgres://` and `mysql://` URLs.
- **Release workflow** now builds with `--features server,duckdb` and includes macOS arm64 in the binary matrix.

### Removed

- `tests/duckdb_integration.rs` — replaced by `tests/parquet_integration.rs`
- `duckdb://` URL detection from `SqlDialect::from_url()`

## [0.4.4] - 2026-04-09

### Added

- **`FormatHint` inference** — every query response now includes a `format_hint` in `ResultMetadata` that tells frontends how to render the result (`LineChart`, `StackedLineChart`, `BarChart`, `Gantt`, `DirectedGraph`, `NodeGraph`, `HorizontalBars`, `ScalarCards`, `LogTable`, `RecordingList`, `Table`)
- **`VisualizationConfig`** — optional `visualization` field in `ResultMetadata` carries axis, series key, color field, and label hints for charting libraries
- **`FORMAT` clause wires through to hints** — explicit `FORMAT table|timeseries|scalar|trace_tree|graph|path` overrides inferred format hint; inference applies automatically when FORMAT is absent
- **`ExecutionError`** — new `ROSQLError` variant wraps all database execution failures with data source context and an actionable `suggestion`; no raw `sqlx`/`duckdb` error text ever reaches the user
- **`CompilerWarning` struct** — replaces `Vec<String>` warnings with structured `{ code, message, suggestion }` objects (e.g. `ANOMALY_NO_FACET`)
- **OTel conventions doc** — `docs/ros2-otel-conventions.md` updated with the full TraceHouse metric set: 17 system metrics, 10 ROS2 runtime metrics, 2 process metrics, deprecated name table, updated ROSQL field alias table
- **Position field mappings** — added `gps.lat`/`gps.lon` aliases, `position.x`/`position.y` aliases, `orientation.yaw` (computed from quaternion), `velocity[N]` and `effort[N]` joint state paths
- **New otel_registry shorthand fields** — `publish_rate`, `bandwidth`, `cpu_usage`, `memory_usage` now point to canonical metric names; 20+ new shorthand aliases for system, ROS2, and process metrics
- **Proto additions** — `result.proto` adds `FormatHint` enum, `VisualizationConfig` message, `CompilerWarning` message, and three new fields on `QueryResult` (field numbers 8, 9, 16)
- **WASM `compile()` response** — now includes `format_hint`, `visualization`, and `warnings` fields
- **WASM `validate()` warnings** — structured `{ code, message, suggestion }` objects instead of raw strings; also surfaces `NOT_IMPLEMENTED` with a code field
- **Cross-repo ticket** — RobotOpsInc/rmw_robotops#42 created for `ros.plan.id` span attribute injection on `/plan` and `/joint_trajectory` spans
- **Documentation** — new `website/docs/concepts.mdx` covering: SHOW vs FROM, scoping with FOR, joint state array indexing, and ANOMALY FACET guidance
- **Cookbook** — added: investigating failed navigation with ENRICH WITH, anomalous robots after deployment, REPL showcase examples (6 queries with expected `format_hint` and `visualization` responses), session/mission data model with Python and C++ examples
- **Command reference** — added FORMAT clause docs, format hint inference table, visualization config field reference, error taxonomy table
- **Schema reference** — updated metric field table with canonical names, expanded position field mappings table, added error taxonomy and warnings section

### Changed

- `ResultMetadata` now carries `format_hint: FormatHint`, `visualization: Option<VisualizationConfig>`, and `warnings: Vec<CompilerWarning>`
- `CompileResult.warnings` changed from `Vec<String>` to `Vec<CompilerWarning>`
- Metric field aliases `cpu_usage` and `memory_usage` now point to canonical OTel metric names (`system.cpu.utilization`, `system.memory.utilization`) instead of legacy names
- `publish_rate` and `bandwidth` now point to `ros2.topic.message_rate` and `ros2.topic.bandwidth` (canonical names)
- `ExecutionError` replaces `DriverError` for post-connection query execution failures; `DriverError` is retained for connection/setup failures only

## [0.4.3] - 2026-04-09

### Added

- **PATH DEVIATION** redesign (replaces reserved stub) — `PATH DEVIATION FOR TRACE 'id'` or `FOR ROBOT 'id' SINCE ...`
  - Optional `PLAN N` selects nth plan (0 = first, -1 = latest, default = -1)
  - Compiles to a three-CTE SQL query: `planned_path` + `actual_poses` + `deviations` with lateral deviation in metres
  - Returns per-waypoint `lateral_deviation_m` plus summary stats (`max_deviation_m`, `avg_deviation_m`, path lengths)
  - Requires `topic_messages` table with `/plan` and `/odom` data
- **JOINT DEVIATION** (new) — `JOINT DEVIATION FOR TRACE 'id'` or `FOR ROBOT 'id' SINCE ...`
  - Compares planned joint trajectory (`/joint_trajectory`) to actual joint states (`/joint_states`)
  - Compiles to a two-CTE SQL query; returns per-joint `position_error_rad` and summary statistics
- **ANOMALY redesign** (replaces reserved stub) — `ANOMALY(<field>) COMPARED TO <baseline> [FACET <field>] [SINCE ...]`
  - `COMPARED TO` is now required (parse error if absent, with suggestion)
  - Optional `FROM <source>` scopes the data source
  - Supported baselines: `last week`, `last 24 hours`, `fleet`
  - Compiles to a two-CTE SQL query: `current_stats` + `baseline_stats`; returns `z_score`, `is_anomalous` (|z| > 2), `direction`
  - Emits a compiler warning if `FACET` is absent
- **WITHIN geospatial operator** — `WHERE <field> WITHIN <radius> OF (<lat>, <lon>)` or `OF POSITION (<x>, <y>)`
  - GPS form uses inline Haversine SQL (great-circle distance)
  - Local frame form uses Euclidean `SQRT(POWER(...))` distance
  - Field path conventions: `position.latitude/longitude` (GPS), `pose.pose.position.x/y` (local)
- **SHOW JOINTS** — `SHOW JOINTS FOR ROBOT 'id'`
  - Returns URDF-derived joint map from `robot_joint_map` table: `joint_name`, `joint_index`, `joint_type`, `lower_limit`, `upper_limit`
- **Array-indexed field access** — `fields['position[0]']` compiles to `"fields"->'position'->>0` (PostgreSQL/DuckDB) or `JSON_EXTRACT(fields, '$.position[0]')` (MySQL)
- **`Baseline::Last24Hours`** — `COMPARED TO last 24 hours` baseline for `ANOMALY()`
- **`DeviationTarget` enum** — `Trace(String)` | `Robot(String)` for scoping `PATH DEVIATION` and `JOINT DEVIATION`
- **`robot_joint_map` table** — new optional schema table for URDF joint metadata (see schema reference)
- **`mcap_metadata.message_types`** column — `JSONB` map of topic → message_type (added to DDL fixture)
- **Completions**: `JOINT DEVIATION`, `SHOW JOINTS`, `WITHIN`, `COMPARED TO`, `last 24 hours` added to autocomplete

### Changed

- `CompileResult` now carries `warnings: Vec<String>` (non-breaking; empty for queries without ANOMALY without FACET)
- `PATH DEVIATION` syntax changed: `FOR TRACE 'id'` or `FOR ROBOT 'id'` now required (was optional bare clause)
- `ANOMALY()` syntax changed: `COMPARED TO <baseline>` now required (was optional)
- Docs (command-reference, cookbook, schema-reference, ros2-otel-conventions) updated in both `website/docs/` and `website/versioned_docs/version-0.4/`
- `SHOW RECORDING` error message updated to suggest `FROM recordings WHERE topic = '...'`

### Notes

- `ros.plan.id` span attribute is needed for full `PATH DEVIATION` correlation — cross-repo ticket required for `rmw_robotops`

## [0.4.2] - 2026-04-09

### Added

- **TIMESERIES** — time-bucketed aggregation clause: `TIMESERIES 5 min` in standard and pipeline queries
  - Compiles to `time_bucket` (DuckDB) / `date_bin` (PostgreSQL) / `FROM_UNIXTIME(... DIV ...)` (MySQL)
  - Automatically prepends `time_bucket` to SELECT, adds to GROUP BY, defaults `ORDER BY time_bucket ASC`
  - Composes with `FACET`: `GROUP BY time_bucket, <facet_col>`
  - Exempt from the default `LIMIT 100` safety cap
- **ENRICH WITH** — cross-source data correlation in two-phase execution
  - `ENRICH WITH <source> [LIMIT N] [SAMPLE FULL]` appends enrichment rows to each primary result row
  - Default 50 enrichment rows per join key; overridable with `LIMIT N`; `SAMPLE FULL` skips the cap
  - Multiple enrichments supported: `ENRICH WITH logs LIMIT 20 ENRICH WITH metrics`
  - Executor uses a window-function SQL pattern (`ROW_NUMBER OVER PARTITION BY`) to respect per-row limits
  - Enrichment rows merged as `_enriched[source]` JSON in result rows; metadata carries `EnrichmentMeta`
- **SHOW TOPICS** — active ROS2 topic summary from span attributes (topic_name, message_type, avg_rate_hz, publishers, subscribers, last_message_age_ms)
- **SHOW NODES** — active ROS2 node summary (node_name, topics_published, topics_subscribed, error_count, last_seen)
- **SHOW NODE GRAPH** — topic/node edges for graph visualisation (source_node, topic, target_node); all three accept `FOR ROBOT` and `SINCE` scoping
- **Completions**: `SHOW TOPICS`, `SHOW NODES`, `SHOW NODE GRAPH`, `TIMESERIES`, `ENRICH WITH` added to autocomplete; `SHOW <cursor>` context now suggests SHOW subcommands
- **Bug fix**: `SINCE 30 min ago` now parses correctly — short-form SI unit abbreviations (`min`, `h`, `s`, `ms`, `ns`, `d`, `w`) were missing from the time-unit recogniser
- **Dialect**: `json_access_text()` wraps JSON extraction in `CAST(... AS VARCHAR)` for DuckDB to avoid type-cast errors in WHERE/ORDER BY/DISTINCT contexts; `time_bucket()` method for all three dialects

### Changed

- `CompileResult` carries `enrichments: Vec<EnrichmentPlan>` (non-breaking; empty for queries without `ENRICH WITH`)
- `ResultMetadata` carries `enrichment_metadata: Vec<EnrichmentMeta>` (non-breaking; empty by default)

## [0.4.1] - 2026-04-09

### Added

- **Universal scoping phase 2+3**: `MESSAGE FLOW FROM TOPIC '...' [TO NODE|TOPIC '...']` replaces removed `MESSAGE PATHS`/`MESSAGE PATH`; compiles to a `msg_flow` recursive CTE
- **TRACE recursive CTE**: `TRACE 'id'` now walks `parent_span_id → span_id` with `WITH RECURSIVE trace_tree`; seeds the CTE with active scope filters
- **SHOW DEPLOYMENTS**: `SHOW DEPLOYMENTS [FOR ROBOT '...'] [SINCE ...]` — deployment history grouped by version + environment
- **SHOW SPAN SUMMARY**: latency report (span_name, count, avg/max duration) grouped by span name
- **SHOW PLANS**: navigation plan spans filtered on `ros.plan.id IS NOT NULL`; accepts `FOR TRACE 'id'` or `FOR ROBOT '...'`
- **COMPARE TO VERSION**: `COMPARE TO VERSION '1.2.3'` baseline; `COMPARE VERSION '...' VERSION '...'` pair baseline

### Removed

- `MESSAGE JOURNEY` — parse now returns a descriptive `ParseError` pointing to `TRACE`
- `MESSAGE PATHS` / `MESSAGE PATH` — parse now returns descriptive `ParseError` messages pointing to `MESSAGE FLOW`

### Changed

- Docs (command-reference, cookbook, schema-reference) updated in both `docs/` and `versioned_docs/version-0.4/`

## [0.4.0] - 2026-04-09

### Added

- **6 aggregation functions** with real SQL compilation: `TOPIC_RATE`, `ACTION_SUCCESS_RATE`, `MOVING_AVG`, `DERIVATIVE`, `APPROX_COUNT_DISTINCT`, `APPROX_PERCENTILE` — all dialect-aware (PostgreSQL / DuckDB / MySQL)
- **`OFFSET` keyword**: `LIMIT N OFFSET M` and pipeline `| OFFSET N`
- **Default `LIMIT 100` safety cap**: applied to all queries except scalar aggregates, `FACET`, `TRACE`, `MESSAGE FLOW`; `default_limit_applied` flag in `ResultMetadata`
- **`ExecOptions::max_rows`** enforced at execution time
- **`ROSQLError::NotImplemented { feature, message }`** variant with workaround guidance
- **Universal scoping phase 1**: `QueryScope` struct; `FOR VERSION '...'`, `FOR ENVIRONMENT '...'`, `FOR SESSION '...'` filter on resource attributes

### Changed

- `COMPARE VERSION` pair baseline compares two robot software versions side by side
- `ALERT`/`DEFINE` reserved syntax errors now surface platform-specific guidance

### Gated (NotImplemented)

- 5 aggregations: `NODE_STATUS`, `EXPECTED`, `UPTIME`, `RATE`, `DELTA` — each returns an actionable workaround
- 6 compound clauses: `HEALTH`, `ANOMALY`, `PATH DEVIATION`, `CORRELATE WITH`, `SHOW RECORDING`, `SHOW TRACE_BREAKDOWN`

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
