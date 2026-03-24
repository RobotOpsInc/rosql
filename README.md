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

ROSQL is a structured query language purpose-built for ROS2 telemetry data stored via [OpenTelemetry](https://opentelemetry.io/). It lets robotics engineers query traces, spans, and metrics using familiar SQL-like syntax with first-class support for ROS2 concepts like nodes, actions, and topic hierarchies. Built in Rust, available as a library, CLI, and WASM package.

## Architecture

### Open source pipeline (self-hosted)

```
  ROS2 System
       │  ros.node, ros.action.*, ParentSpanId
       ▼
  OTel Collector (opentelemetry-collector-contrib)
       │  OTLP → SQL exporter
       ▼
  SQL-compatible DB (others supported soon)
       │  OTel standard schema
       ▼
  rosql driver (SQLBackend / others supported soon)
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
  │ Mode 2: WASM (frontend editor)                      │
  │   parse() / validate() / get_completions()          │
  └─────────────────────────────────────────────────────┘
```

## Quick start

Add ROSQL to your `Cargo.toml`:

```toml
[dependencies]
rosql = "0.1"
```

Parse and execute a query against any SQL-compatible database:

```rust
use rosql::{SQLBackend, ROSQLEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = SQLBackend::connect("postgresql://localhost/telemetry")?;
    let engine = ROSQLEngine::new(backend);

    let results = engine.execute("
        SELECT span_name, duration
        FROM spans
        WHERE ros.node = '/navigation/planner'
        ORDER BY duration DESC
        LIMIT 10
    ")?;

    println!("{results}");
    Ok(())
}
```

See [`examples/`](examples/) for full working examples with fixture data.

## Driver support

| Driver | Feature flag | Backend | Auth | Status |
|--------|-------------|---------|------|--------|
| ANSI SQL | *(default)* | `SQLBackend` | Connection string | v0.1 |
| PostgreSQL | `postgres` | `PostgresBackend` | Connection string | Coming soon |
| DuckDB | `duckdb` | `DuckDbBackend` | None (local file) | Coming soon |
| Pandas | `pandas` | `PandasBackend` | None (in-process) | Coming soon |
| InfluxDB | `influxdb` | `InfluxDbBackend` | Token / connection string | Coming soon |
| Athena | `athena` | `AthenaBackend` | AWS IAM | Coming soon |
| BigQuery | `bigquery` | `BigQueryBackend` | GCP service account | Coming soon |

The default ANSI SQL driver works with any standard SQL-compatible database out of the box — PostgreSQL, MySQL, TimescaleDB, and others that support standard SQL over a connection string.

## Performance

Benchmarks are in progress (Mar 2026). See [BENCHMARKS.md](BENCHMARKS.md) once available, and [#6](https://github.com/RobotOpsInc/rosql/issues/6) for test infrastructure tracking.

## Schema

ROSQL expects telemetry data stored in the [OpenTelemetry schema conventions for ROS2](docs/ros2-otel-conventions.md). This includes standard OTel span and trace tables augmented with ROS2-specific resource and span attributes (`ros.node`, `ros.action.*`, etc.).

See the [full schema expectations doc](docs/ros2-otel-conventions.md) for required tables, columns, and attribute mappings.

## Documentation and reference

- [ROSQL Language Spec](https://rosql.org/spec) — full grammar, keywords, and semantics
- [Driver Architecture](https://rosql.org/drivers) — how backends translate AST to SQL dialects

## Examples

```sql
-- Find the slowest action server callbacks in the last hour
SELECT ros.node, ros.action.name, span_name, duration
FROM spans
WHERE ros.action.name IS NOT NULL
  AND start_time > now() - INTERVAL '1 hour'
ORDER BY duration DESC
LIMIT 20;

-- Trace a full action lifecycle by parent span
SELECT span_name, status, duration, ParentSpanId
FROM spans
WHERE TraceId = 'abc123'
ORDER BY start_time;

-- Aggregate latency by node
SELECT ros.node, AVG(duration) AS avg_duration, COUNT(*) AS call_count
FROM spans
GROUP BY ros.node
ORDER BY avg_duration DESC;

-- Validate topic publish rates
SELECT ros.topic, COUNT(*) / EXTRACT(EPOCH FROM MAX(end_time) - MIN(start_time)) AS msgs_per_sec
FROM spans
WHERE ros.topic IS NOT NULL
GROUP BY ros.topic;
```

See the [`examples/`](examples/) directory for runnable demos ([#7](https://github.com/RobotOpsInc/rosql/issues/7)).

## WASM API

The `@robotops/rosql` npm package exposes parsing and validation for use in browser editors and tooling:

```typescript
import { parse, validate, getCompletions } from '@robotops/rosql';

const ast = parse('SELECT span_name FROM spans WHERE ros.node = "/planner"');
console.log(ast);

const errors = validate('SELECT bad_column FROM not_a_table');
console.log(errors);

const completions = getCompletions('SELECT span_name FROM spans WHERE ros.');
console.log(completions);
```

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
cargo build --release
```

## Local development

```sh
git clone https://github.com/RobotOpsInc/rosql
cd rosql
just build       # build default features
just test        # run tests
just build-wasm  # build WASM package
```

## Guidelines for contributing

We want your help! ROSQL is in early development (v0.1) and contributions are welcome.

- See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow and coding standards
- File bugs and feature requests in the [issue tracker](https://github.com/RobotOpsInc/rosql/issues)
- Questions or ideas? Reach out at [kristophm@robotops.com](mailto:kristophm@robotops.com)

## Robot Ops platform

For fleet-scale telemetry with managed ingestion, storage, and dashboards, see the [Robot Ops platform](https://robotops.com).

## License

Apache 2.0 — see [LICENSE](LICENSE).
