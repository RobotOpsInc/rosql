# ROSQL Examples

End-to-end examples with realistic ROS2 telemetry fixture data.

## Structure

```
examples/
├── queries/                    ← ROSQL query files (shared across all backends)
│   ├── basic.rosql
│   ├── compound_clauses.rosql
│   ├── pipeline.rosql
│   └── timeseries.rosql
├── postgres/                   ← PostgreSQL backend (full fixtures + Docker)
│   ├── docker-compose.yml
│   ├── fixtures/
│   └── README.md
├── mysql/                      ← MySQL backend (guide)
│   └── README.md
└── README.md                   ← this file
```

Query files are backend-agnostic — the same `.rosql` queries work against any supported database. Each backend subfolder contains setup instructions and (where applicable) fixture data.

## Backends

### PostgreSQL

The primary example backend with full Docker Compose setup and fixture data. See [postgres/](postgres/).

```sh
just examples-up
cargo run --features server,postgres --bin rosql -- query \
  "FROM traces WHERE status = 'ERROR'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples
just examples-down
```

### MySQL

For organizations with existing MySQL infrastructure. See [mysql/](mysql/).

```sh
cargo run --features server,mysql --bin rosql -- query \
  "FROM traces WHERE status = 'ERROR'" \
  --backend mysql --url mysql://user:pass@localhost:3306/telemetry
```

### DuckDB (coming soon)

See [#18](https://github.com/RobotOpsInc/rosql/issues/18).

## What the fixture data represents

A simulated ROS2 robot (`robot_sim_001`) running Nav2 performs three navigation actions:

| Action | Goal | Result | Duration | What happens |
|--------|------|--------|----------|-------------|
| 1 | Waypoint A (5, 3) | Success | ~8s | Clean navigation, smooth path |
| 2 | Waypoint B (10, 7) | **Aborted** | ~12s | Battery drops to 15%, CPU spikes to 92%, path deviates |
| 3 | Waypoint C (15, 2) | **Timed out** | ~30s | Controller makes no progress |

## Schema profiles

Different OTel Collector exporters use different column naming conventions:

```sh
# OTel PostgreSQL exporter (lowercase — default)
cargo run --features server --bin rosql -- compile \
  "FROM traces WHERE status = 'ERROR'" --backend postgres --schema otel-postgres

# OTel ClickHouse exporter (PascalCase)
cargo run --features server --bin rosql -- compile \
  "FROM traces WHERE status = 'ERROR'" --backend postgres --schema otel-clickhouse
```

See [#22](https://github.com/RobotOpsInc/rosql/issues/22) for custom schema profile support.

## Running tests

Parse + compile tests for all example queries run in CI:
```sh
cargo test   # includes example query tests
```

Full end-to-end tests against PostgreSQL:
```sh
just test-examples
```
