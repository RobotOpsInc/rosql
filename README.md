<p align="center">
  <h1 align="center">ROSQL™</h1>
  <p align="center"><em>The open source query language that natively speaks robot</em></p>
</p>

<p align="center">
  <a href="https://crates.io/crates/rosql"><img src="https://img.shields.io/crates/v/rosql.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/@robotops/rosql"><img src="https://img.shields.io/npm/v/@robotops/rosql.svg" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License"></a>
  <a href="https://github.com/RobotOpsInc/rosql/actions"><img src="https://github.com/RobotOpsInc/rosql/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

---

**ROSQL** (pronounced *"RAW-skul"*) is **Robot Ops Structured Query Language** — a SQL-like language and driver purpose-built for ROS2 telemetry data stored via [OpenTelemetry](https://opentelemetry.io/). It lets robotics engineers query traces, logs, and metrics using familiar SQL-like syntax with first-class support for ROS2 concepts — nodes, actions, topics, and message causality.

Robot observability is hard. ROS2 systems generate a firehose of traces, logs, and sensor data across dozens of nodes, but general-purpose query languages have no awareness of topics, action graphs, or message causality. ROSQL closes that gap: write queries in the language of your robot, not your database.

Built in Rust and available as a library, CLI, gRPC server, and WASM package, ROSQL is created and used by [Robot Ops, Inc.](https://robotops.com) to power the [Robot Ops observability platform](https://robotops.com).

> **Docs, cookbook, and live demo → [rosql.org](https://rosql.org)**

> ROSQL is a trademark of Robot Ops, Inc.

## Architecture

```
  ROS2 System
       │
       │  OTel attributes (ros.node, ros.action.*, ros.topic, ParentSpanId)
       ▼
  Robot Ops Agent (robotops.com) or OTel Collector (community)
       │
       │  OTLP gRPC
       ▼
  Datastore (e.g. PostgreSQL + TimescaleDB, ClickHouse, or any SQL-compatible DB)
       │
       │  OTel standard schema
       ▼
  rosql (parse + compile + execute)
       │
       ▼
  Query results
```

### Usage modes

```
  ┌─────────────────────────────────────────────────────┐
  │ Mode 1: Parse + Execute (library)                   │
  │   ROSQL text → parser → AST → ROSQLBackend → DB    │
  ├─────────────────────────────────────────────────────┤
  │ Mode 2: CLI + gRPC Server                           │
  │   rosql parse / compile / query / serve             │
  ├─────────────────────────────────────────────────────┤
  │ Mode 3: WASM (frontend editor)                      │
  │   parse() / validate() / get_completions()          │
  └─────────────────────────────────────────────────────┘
```

## Quick start

### What ROSQL looks like

Express in one sentence what would take a page of SQL: find every navigation failure that happened while the battery was critically low.

```sql
SELECT trace_id, span_name_col, service_name, duration, status_code, span_attributes
FROM traces
WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'
DURING(
  FROM topics WHERE topic_name = '/battery_state'
  AND fields['percentage'] < 15
)
SINCE 6 hours ago
```

```json
{
  "columns": ["trace_id", "span_name_col", "service_name", "duration", "status_code", "span_attributes"],
  "rows": [
    ["a3f1c9d2e8b04f7a", "navigate_to_pose", "bt_navigator", 1423187000, "ERROR",
      {"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "aborted"}],
    ["c7d2f1a3b9e54c2b", "navigate_to_pose", "bt_navigator",  873456000, "ERROR",
      {"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "aborted"}]
  ],
  "metadata": { "rows_returned": 2, "elapsed_ms": 14 }
}
```

> `duration` is stored as nanoseconds (`BIGINT`). 1423187000 ns ≈ 1.42 s.

Or trace the full message causality chain from a single span — something SQL has no primitive for. `TRACE 'id'` walks `parent_span_id → span_id` recursively and returns all columns from `otel_traces`:

```sql
TRACE 'a3f1c9d2e8b04f7a'
```

```json
{
  "columns": ["timestamp", "trace_id", "span_id", "parent_span_id", "span_name_col", "span_kind", "service_name", "duration", "status_code", "span_attributes", "resource_attributes"],
  "rows": [
    ["2025-03-25T14:32:11.012Z", "a3f1c9d2e8b04f7a", "f0e1d2c3b4a59687", "",                 "goal_received",     "INTERNAL", "bt_navigator",       0,         "OK",    {"ros.node": "/bt_navigator"},                                       {"host.name": "robot_03"}],
    ["2025-03-25T14:32:11.013Z", "a3f1c9d2e8b04f7a", "1a2b3c4d5e6f7089", "f0e1d2c3b4a59687", "compute_path",      "INTERNAL", "planner_server",    38000000,  "OK",    {"ros.node": "/planner_server",    "ros.topic": "/plan"},              {"host.name": "robot_03"}],
    ["2025-03-25T14:32:11.051Z", "a3f1c9d2e8b04f7a", "9876543210abcdef", "1a2b3c4d5e6f7089", "follow_path",       "INTERNAL", "controller_server", 412000000, "ERROR", {"ros.node": "/controller_server", "ros.topic": "/cmd_vel"},            {"host.name": "robot_03"}],
    ["2025-03-25T14:32:11.463Z", "a3f1c9d2e8b04f7a", "fedcba9876543210", "9876543210abcdef", "obstacle_detected", "INTERNAL", "local_costmap",       6000000, "OK",    {"ros.node": "/local_costmap",     "ros.topic": "/costmap_update"},    {"host.name": "robot_03"}]
  ],
  "metadata": { "rows_returned": 4, "elapsed_ms": 3 }
}
```

### As a CLI

```sh
# Install (Linux x86_64 / arm64, macOS Apple Silicon)
curl -fsSL https://rosql.org/install.sh | sh

# Or build from source (all platforms)
cargo install rosql --features server,duckdb

# Query local Parquet telemetry files
rosql query "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend parquet --url ./telemetry/robotops_demo_agent/20260403-141530/

# Query Parquet files on S3
rosql query "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend parquet --url s3://my-bucket/robot-01/robotops_demo_agent/20260403-141530/

# Query a PostgreSQL database
rosql query "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend postgres --url postgresql://user:pass@localhost:5432/telemetry

# Compile to SQL (inspect what ROSQL generates — no DB or --url needed)
rosql compile "FROM traces WHERE duration > 500 ms" --backend parquet

# Trace a message causality chain
rosql query "TRACE 'a3f1c9d2e8b04f7a'" \
  --backend postgres --url postgresql://user:pass@localhost:5432/telemetry
```

### As a library

```sh
cargo add rosql
```

```rust
use rosql::{parse, drivers::{SqlBackend, ExecOptions}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let backend = SqlBackend::new("postgresql://user:pass@localhost/telemetry").await?;
    let query = parse("
        SELECT trace_id, span_name_col, service_name, duration, status_code
        FROM traces WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'
        DURING(
          FROM topics WHERE topic_name = '/battery_state' AND fields['percentage'] < 15
        )
        SINCE 6 hours ago
    ")?;
    let result = backend.execute(&query, &ExecOptions::default()).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
```

## Driver support

| Backend | Feature flag | Status |
|---------|-------------|--------|
| <img src="https://img.shields.io/badge/PostgreSQL-4169E1?logo=postgresql&logoColor=white" alt="PostgreSQL" height="20"> PostgreSQL / TimescaleDB | `postgres` | ![v0.1](https://img.shields.io/badge/v0.1-green) |
| <img src="https://img.shields.io/badge/MySQL-4479A1?logo=mysql&logoColor=white" alt="MySQL" height="20"> MySQL / MariaDB | `mysql` | ![v0.1](https://img.shields.io/badge/v0.1-green) |
| <img src="https://img.shields.io/badge/DuckDB-FFF000?logo=duckdb&logoColor=black" alt="DuckDB" height="20"> Parquet (local / S3) powered by DuckDB | `duckdb` | ![v0.4.5](https://img.shields.io/badge/v0.4.5-green) |
| <img src="https://img.shields.io/badge/AWS_Athena-232F3E?logo=amazonaws&logoColor=white" alt="Athena" height="20"> AWS Athena | `athena` | [![Future](https://img.shields.io/badge/future-lightgrey)](https://github.com/RobotOpsInc/rosql/issues/9) |
| <img src="https://img.shields.io/badge/BigQuery-4285F4?logo=googlebigquery&logoColor=white" alt="BigQuery" height="20"> Google BigQuery | `bigquery` | [![Future](https://img.shields.io/badge/future-lightgrey)](https://github.com/RobotOpsInc/rosql/issues/10) |

## Feature flags

| Feature | What it enables | Dependencies |
|---------|----------------|--------------|
| *(default)* | Parser, AST, unit system, SQL compiler, proto types | logos, serde, prost |
| `postgres` | PostgreSQL / TimescaleDB driver | sqlx, tokio |
| `mysql` | MySQL / MariaDB driver | sqlx, tokio |
| `duckdb` | Parquet file backend (local + S3) powered by DuckDB — use with `--backend parquet` | duckdb (bundled), tokio |
| `server` | `rosql` CLI binary + gRPC server | tonic, tokio, clap |
| `wasm` | WASM exports for frontend editors | wasm-bindgen |

## CLI

```sh
# Parse → JSON AST
rosql parse "FROM traces WHERE duration > 500 ms SINCE 1 hour ago"

# Compile → SQL (shows what SQL ROSQL generates — no DB or --url needed)
rosql compile "FROM traces WHERE duration > 500 ms" --backend parquet
rosql compile "FROM traces WHERE duration > 500 ms" --backend postgres

# Execute → query results as JSON (Parquet backend — local or S3)
rosql query "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend parquet --url ./telemetry/robotops_demo_agent/20260403-141530/

rosql query "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend parquet --url s3://my-bucket/robot-01/robotops_demo_agent/20260403-141530/

# Execute → query results as JSON (PostgreSQL)
rosql query "FROM traces WHERE status = 'ERROR'" \
  --backend postgres --url postgresql://user:pass@localhost:5432/db

# Validate syntax
rosql validate "SELECT * FROM logs"

# Autocomplete suggestions at cursor position
rosql completions "FROM " 5

# Start gRPC server on Unix socket
rosql serve --socket /tmp/rosql.sock

# Schema profiles (match your OTel Collector exporter):
#   --schema otel-postgres    (lowercase columns, default)
#   --schema otel-clickhouse  (PascalCase columns)
```

### Parquet backend: expected directory layout

The `--backend parquet --url <path>` option expects the following directory structure (matching the [demo-agent output format](https://github.com/RobotOpsInc/rmw_robotops/tree/development/demo-agent#output-format)):

```
<url>/
  traces/          *.parquet  →  otel_traces view
  logs/            *.parquet  →  otel_logs view
  metrics/         *.parquet  →  otel_metrics view
  topic_messages/  *.parquet  →  topic_messages view
  mcap_metadata/   *.parquet  →  mcap_metadata view
```

Files are discovered recursively via `**/*.parquet` globs. Missing subdirectories are silently skipped — queries against absent tables return a `DataSourceUnavailable` error.

### S3 credentials for the Parquet backend

When `--url s3://...` is used, ROSQL loads the DuckDB `httpfs` extension and reads credentials from standard AWS environment variables:

| Variable | Description |
|---|---|
| `AWS_ACCESS_KEY_ID` | Static access key |
| `AWS_SECRET_ACCESS_KEY` | Static secret key |
| `AWS_REGION` / `AWS_DEFAULT_REGION` | AWS region (e.g. `us-east-1`) |
| `AWS_PROFILE` | Named credentials profile |
| `AWS_ENDPOINT_URL` | Override endpoint for S3-compatible storage (MinIO, Ceph, etc.) |

## Examples

```sql
-- Find slow navigation actions
SELECT span_name, duration
FROM traces
WHERE action_name = '/navigate_to_pose' AND duration > 500 ms
SINCE 1 hour ago
ORDER BY duration DESC
LIMIT 20

-- Cross-signal correlation: errors during low battery
FROM traces WHERE status = 'ERROR'
DURING(
  FROM topics WHERE topic_name = '/battery_state'
  AND fields['percentage'] < 20
)
SINCE yesterday

-- Time-bucketed error rate dashboard (TIMESERIES)
SELECT COUNT(*) AS errors FROM traces WHERE status = 'ERROR'
TIMESERIES 5 min
SINCE 1 hour ago
FACET robot_id

-- Enrich failed navigation traces with log context (ENRICH WITH)
SELECT * FROM traces
WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'
SINCE 1 hour ago
ENRICH WITH logs LIMIT 20

-- System topology: active topics and nodes
SHOW TOPICS FOR ROBOT 'robot_42' SINCE 30 minutes ago
SHOW NODES FOR ROBOT 'robot_42' SINCE 30 minutes ago
SHOW NODE GRAPH FOR ROBOT 'robot_42' SINCE 30 minutes ago

-- Trace span tree walk
TRACE 'abc123def456'

-- Pipeline syntax
FROM traces
| WHERE duration > 500 ms
| TIMESERIES 1 min
| FACET robot_id
```

See [`examples/`](examples/) for a full walkthrough with Docker Compose, PostgreSQL fixture data, and runnable queries.

### Cookbook: Investigating a failed navigation with enriched logs

When a navigation action fails, the recommended workflow is:

```sql
-- 1. Find the failing trace
SELECT trace_id, span_name, duration
FROM traces
WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'
SINCE 30 min ago
ORDER BY duration DESC
LIMIT 5

-- 2. Walk the full causality chain for the worst offender
TRACE 'your-trace-id-here'

-- 3. Correlate with log output for that same window
SELECT * FROM traces
WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'
SINCE 30 min ago
ENRICH WITH logs LIMIT 50

-- 4. Check which topics were active during the failure
SHOW TOPICS FOR ROBOT 'robot_42' SINCE 30 min ago

-- 5. See the node graph to spot missing pub/sub edges
SHOW NODE GRAPH FOR ROBOT 'robot_42' SINCE 30 min ago
```

## WASM API

The `@robotops/rosql` npm package exposes parsing and validation for browser editors:

```typescript
import init, { parse, validate, get_completions } from '@robotops/rosql';

await init();

const result = parse('FROM traces WHERE duration > 500 ms SINCE 1 hour ago');
console.log(result);

const errors = validate('INSERT INTO logs');
console.log(errors); // { valid: false, errors: [...] }

const completions = get_completions('FROM ', 5);
console.log(completions); // [{ label: 'logs', ... }, { label: 'traces', ... }, ...]
```

## Schema

ROSQL expects telemetry data in the [OpenTelemetry schema conventions for ROS2](docs/ros2-otel-conventions.md). This includes standard OTel tables (`otel_traces`, `otel_logs`, `otel_metrics`) with ROS2-specific span attributes (`ros.node`, `ros.action.*`, `ros.topic`).

## Performance

See [BENCHMARKS.md](BENCHMARKS.md) for performance data (coming soon — [#35](https://github.com/RobotOpsInc/rosql/issues/35)).

## Installation

### One-liner (Linux x86_64 / arm64, macOS Apple Silicon)

```sh
curl -fsSL https://rosql.org/install.sh | sh
```

Pre-built binaries include the Parquet backend (`--backend parquet`) and are available for:

| Platform | Architecture | Notes |
|----------|-------------|-------|
| Linux | x86_64 | — |
| Linux | aarch64 | — |
| macOS | arm64 (Apple Silicon) | — |
| macOS | x86_64 (Intel) | Build from source (see below) |
| Windows | any | Build from source (see below) |

### Build from source

```sh
cargo install rosql --features server,duckdb   # Parquet + CLI (recommended)
cargo install rosql --features server,postgres  # PostgreSQL + CLI
cargo install rosql --features server           # CLI only (compile/parse/validate)
```

Or clone and build locally:

```sh
git clone https://github.com/RobotOpsInc/rosql
cd rosql

# Library only (default)
cargo build --release

# CLI with Parquet backend (--backend parquet)
cargo build --release --features server,duckdb --bin rosql

# CLI with PostgreSQL
cargo build --release --features server,postgres --bin rosql
```

## Local development

```sh
git clone https://github.com/RobotOpsInc/rosql
cd rosql
just build       # build default features
just test        # run tests
just build-wasm  # build WASM package
just check       # full CI: build + test + clippy + fmt + buf-lint
```

Prerequisites: Rust (stable, 1.80+), protoc, buf (optional). See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Contributing

ROSQL is in early development (v0.1) and contributions are welcome.

- See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow, build variants, and release process
- File bugs and feature requests in the [issue tracker](https://github.com/RobotOpsInc/rosql/issues)
- Questions? Email [kristophm@robotops.com](mailto:kristophm@robotops.com)

## Robot Ops platform

For fleet-scale telemetry with managed ingestion, storage, and dashboards — including lifecycle anchors, fleet-wide anomaly detection, and ClickHouse performance — see the [Robot Ops platform](https://robotops.com).

## License

Apache 2.0 — see [LICENSE](LICENSE).
