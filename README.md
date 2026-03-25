<p align="center">
  <h1 align="center">ROSQL</h1>
  <p align="center"><em>The query language for ROS2 telemetry data</em></p>
</p>

<p align="center">
  <a href="https://crates.io/crates/rosql"><img src="https://img.shields.io/crates/v/rosql.svg" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/@robotops/rosql"><img src="https://img.shields.io/npm/v/@robotops/rosql.svg" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License"></a>
  <a href="https://github.com/RobotOpsInc/rosql/actions"><img src="https://github.com/RobotOpsInc/rosql/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

---

ROSQL is a structured query language purpose-built for ROS2 telemetry data stored via [OpenTelemetry](https://opentelemetry.io/). It lets robotics engineers query traces, logs, and metrics using familiar SQL-like syntax with first-class support for ROS2 concepts like nodes, actions, and topic hierarchies. Built in Rust, available as a library, CLI, gRPC server, and WASM package.

## Architecture

### Pipeline

```
  ROS2 System
       │  ros.node, ros.action.*, ParentSpanId
       ▼
  OTel Collector (opentelemetry-collector-contrib)
       │  OTLP → SQL exporter
       ▼
  SQL-compatible DB (PostgreSQL, SQLite, MySQL)
       │  OTel standard schema
       ▼
  rosql (parse + execute via ROSQLBackend)
       │
       ▼
  Query results
```

### Usage modes

```
  ┌─────────────────────────────────────────────────────┐
  │ Mode 1: Parse + Execute (standalone)                │
  │   ROSQL text → parser → AST → ROSQLBackend → DB    │
  ├─────────────────────────────────────────────────────┤
  │ Mode 2: gRPC Server + CLI                           │
  │   rosql serve / parse / validate             │
  ├─────────────────────────────────────────────────────┤
  │ Mode 3: WASM (frontend editor)                      │
  │   parse() / validate() / get_completions()          │
  └─────────────────────────────────────────────────────┘
```

## Quick start

Add ROSQL to your `Cargo.toml`:

```toml
[dependencies]
rosql = "0.1"
```

Parse a query:

```rust
use rosql::parse;

fn main() {
    let ast = parse("
        SELECT span_name, duration
        FROM traces
        WHERE node = '/navigation/planner'
        ORDER BY duration DESC
        LIMIT 10
    ").unwrap();

    println!("{ast:?}");
}
```

## Feature flags

| Feature | What it enables | Dependencies |
|---------|----------------|--------------|
| *(default)* | Parser, AST, unit system, SQL compiler, proto types | logos, serde, prost |
| `postgres` | PostgreSQL / TimescaleDB driver | sqlx, tokio |
| `mysql` | MySQL / MariaDB driver | sqlx, tokio |
| `server` | `rosql` gRPC server + CLI binary | tonic, tokio, clap |
| `wasm` | WASM exports for frontend editors | wasm-bindgen |
| `duckdb` | DuckDB driver (coming soon — [#18](https://github.com/RobotOpsInc/rosql/issues/18)) | duckdb |

## CLI

Build and use the `rosql` CLI:

```sh
cargo build --features server --bin rosql

# Parse a query to JSON AST
rosql parse "FROM traces WHERE duration > 500 ms SINCE 1 hour ago"

# Validate a query
rosql validate "SELECT * FROM logs"

# Get completions at cursor position
rosql completions "FROM " 5

# Start gRPC server on a Unix socket
rosql serve --socket /tmp/rosql.sock

# Schema profiles (match your OTel Collector exporter):
#   --schema otel-postgres    (lowercase columns, default)
#   --schema otel-clickhouse  (PascalCase columns)
```

## WASM API

The `@robotops/rosql` npm package exposes parsing and validation for use in browser editors:

```typescript
import init, { parse, validate, get_completions } from '@robotops/rosql';

await init();

const result = parse('FROM traces WHERE duration > 500 ms SINCE 1 hour ago');
console.log(result);

const errors = validate('SELECT bad_column FROM not_a_table');
console.log(errors);

const completions = get_completions('FROM ', 5);
console.log(completions);
```

## Examples

```sql
-- Find slow navigation actions in the last hour
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

See the [`examples/`](examples/) directory for runnable demos ([#7](https://github.com/RobotOpsInc/rosql/issues/7)).

## Schema

ROSQL expects telemetry data stored in the [OpenTelemetry schema conventions for ROS2](docs/ros2-otel-conventions.md). This includes standard OTel tables (`otel_traces`, `otel_logs`, `otel_metrics`) augmented with ROS2-specific span attributes (`ros.node`, `ros.action.*`, etc.).

## Options for running

### Pre-built binaries

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | [GitHub Releases](https://github.com/RobotOpsInc/rosql/releases) |
| Linux | aarch64 | [GitHub Releases](https://github.com/RobotOpsInc/rosql/releases) |

### Building from source

```sh
git clone https://github.com/RobotOpsInc/rosql
cd rosql

# Library only (default)
cargo build --release

# gRPC server + CLI binary
cargo build --release --features server --bin rosql
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

Prerequisites: Rust (stable), protoc, buf (optional). See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Guidelines for contributing

We want your help! ROSQL is in early development (v0.1) and contributions are welcome.

- See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow and coding standards
- File bugs and feature requests in the [issue tracker](https://github.com/RobotOpsInc/rosql/issues)
- Questions or ideas? Reach out at [kristophm@robotops.com](mailto:kristophm@robotops.com)

## Robot Ops platform

For fleet-scale telemetry with managed ingestion, storage, and dashboards, see the [Robot Ops platform](https://robotops.com).

## License

Apache 2.0 — see [LICENSE](LICENSE).
