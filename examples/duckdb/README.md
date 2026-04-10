# ROSQL × DuckDB SQL Fixtures

This directory contains DuckDB-compatible SQL fixture files used to populate integration test databases and to generate the Parquet fixtures used by `--backend parquet`.

## Fixture files

| File | Content |
|------|---------|
| `01_schema.sql` | Table DDL (`otel_traces`, `otel_logs`, `otel_metrics`, `topic_messages`, `mcap_metadata`) |
| `02_traces.sql` | 3 navigation actions for `robot_sim_001` (success, abort, timeout) |
| `03_logs.sql` | `/rosout` log entries for each navigation action |
| `04_metrics.sql` | Topic rates and system resource metrics |
| `05_topic_messages.sql` | `/plan`, `/odom`, and `/battery_state` topic messages |
| `06_mcap_metadata.sql` | One MCAP recording session |

## Schema notes

These fixtures use DuckDB-compatible column types:
- `JSONB` → `JSON` (DuckDB does not have a JSONB type)
- `TEXT[]` → `VARCHAR[]`

See [`01_schema.sql`](fixtures/01_schema.sql) for the full DDL.

## Using the Parquet backend

The recommended way to query these fixtures is via the Parquet backend, using the pre-built Parquet files in `examples/parquet/fixtures/`:

```bash
# Build with Parquet support
cargo build --features server,duckdb --bin rosql

# Query the Parquet fixtures
cargo run --features server,duckdb --bin rosql -- query \
  "FROM traces WHERE status = 'ERROR'" \
  --backend parquet --url ./examples/parquet/fixtures/

cargo run --features server,duckdb --bin rosql -- query \
  "TRACE 'trace-002'" \
  --backend parquet --url ./examples/parquet/fixtures/

cargo run --features server,duckdb --bin rosql -- query \
  "SHOW TOPICS SINCE 30 days ago" \
  --backend parquet --url ./examples/parquet/fixtures/
```

See [`examples/parquet/`](../parquet/) for more details and to regenerate Parquet files from these SQL sources.

## Compile ROSQL → DuckDB SQL

The `compile` subcommand shows the SQL ROSQL generates for DuckDB (no `--url` needed):

```bash
cargo run --features server --bin rosql -- compile \
  "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend parquet
```

The output uses DuckDB-specific functions: `NOW()::TIMESTAMP` for interval arithmetic,
`approx_count_distinct()`, `approx_quantile()`, `time_bucket()`, etc.

## Running integration tests

```bash
cargo test --features duckdb
```

The Parquet integration tests run without Docker — they read the pre-built Parquet fixtures directly.
