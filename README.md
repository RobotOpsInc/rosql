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

### As a library

```sh
cargo add rosql
```

```rust
use rosql::parse;

fn main() {
    let ast = parse("
        SELECT span_name, duration
        FROM traces
        WHERE node = '/navigation/planner'
          AND duration > 500 ms
        SINCE 1 hour ago
        ORDER BY duration DESC
        LIMIT 10
    ").unwrap();

    println!("{ast:?}");
}
```

### As a CLI

```sh
# Build the CLI
cargo build --features server,postgres --bin rosql

# Parse a query to JSON AST
rosql parse "FROM traces WHERE status = 'ERROR'"

# Compile to SQL (no database needed)
rosql compile "FROM traces WHERE duration > 500 ms" --backend postgres

# Execute against a database
rosql query "FROM traces WHERE status = 'ERROR'" \
  --backend postgres --url postgresql://user:pass@localhost:5432/telemetry
```

## Driver support

| Backend | Feature flag | Status |
|---------|-------------|--------|
| <img src="https://img.shields.io/badge/PostgreSQL-4169E1?logo=postgresql&logoColor=white" alt="PostgreSQL" height="20"> PostgreSQL / TimescaleDB | `postgres` | ![v0.1](https://img.shields.io/badge/v0.1-green) |
| <img src="https://img.shields.io/badge/MySQL-4479A1?logo=mysql&logoColor=white" alt="MySQL" height="20"> MySQL / MariaDB | `mysql` | ![v0.1](https://img.shields.io/badge/v0.1-green) |
| <img src="https://img.shields.io/badge/DuckDB-FFF000?logo=duckdb&logoColor=black" alt="DuckDB" height="20"> DuckDB (embedded) | `duckdb` | [![Coming soon](https://img.shields.io/badge/coming_soon-yellow)](https://github.com/RobotOpsInc/rosql/issues/18) |
| <img src="https://img.shields.io/badge/AWS_Athena-232F3E?logo=amazonaws&logoColor=white" alt="Athena" height="20"> AWS Athena | `athena` | [![Future](https://img.shields.io/badge/future-lightgrey)](https://github.com/RobotOpsInc/rosql/issues/9) |
| <img src="https://img.shields.io/badge/BigQuery-4285F4?logo=googlebigquery&logoColor=white" alt="BigQuery" height="20"> Google BigQuery | `bigquery` | [![Future](https://img.shields.io/badge/future-lightgrey)](https://github.com/RobotOpsInc/rosql/issues/10) |

## Feature flags

| Feature | What it enables | Dependencies |
|---------|----------------|--------------|
| *(default)* | Parser, AST, unit system, SQL compiler, proto types | logos, serde, prost |
| `postgres` | PostgreSQL / TimescaleDB driver | sqlx, tokio |
| `mysql` | MySQL / MariaDB driver | sqlx, tokio |
| `server` | `rosql` CLI binary + gRPC server | tonic, tokio, clap |
| `wasm` | WASM exports for frontend editors | wasm-bindgen |

## CLI

```sh
# Parse → JSON AST
rosql parse "FROM traces WHERE duration > 500 ms SINCE 1 hour ago"

# Compile → SQL (shows what SQL ROSQL generates)
rosql compile "FROM traces WHERE duration > 500 ms" --backend postgres

# Execute → query results as JSON
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

-- Message causality graph
MESSAGE JOURNEY FOR TRACE 'abc123def456'

-- Robot health assessment
HEALTH() FOR ROBOT 'robot_42' SINCE 30 minutes ago

-- Pipeline syntax
FROM traces
| WHERE duration > 500 ms
| FACET robot_id
| COMPARE TO last week
```

See [`examples/`](examples/) for a full walkthrough with Docker Compose, PostgreSQL fixture data, and runnable queries.

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

## Pre-built binaries

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | [GitHub Releases](https://github.com/RobotOpsInc/rosql/releases) |
| Linux | aarch64 | [GitHub Releases](https://github.com/RobotOpsInc/rosql/releases) |

Other platforms: build from source.

### Building from source

```sh
git clone https://github.com/RobotOpsInc/rosql
cd rosql

# Library only (default)
cargo build --release

# CLI binary
cargo build --release --features server --bin rosql

# CLI with PostgreSQL query execution
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
