# ROSQL × Parquet Backend

Query Parquet telemetry files using `--backend parquet`. No Docker or external server required — DuckDB runs embedded and reads Parquet files directly from disk or S3.

## Quick start

```bash
# Build with Parquet support
cargo build --features server,duckdb --bin rosql

# Query the example Parquet fixtures
cargo run --features server,duckdb --bin rosql -- query \
  "FROM traces WHERE status = 'ERROR'" \
  --backend parquet --url ./examples/parquet/fixtures/

cargo run --features server,duckdb --bin rosql -- query \
  "SHOW TOPICS SINCE 30 days ago" \
  --backend parquet --url ./examples/parquet/fixtures/

cargo run --features server,duckdb --bin rosql -- query \
  "SELECT COUNT(*) FROM traces TIMESERIES 1 hour SINCE 30 days ago" \
  --backend parquet --url ./examples/parquet/fixtures/
```

## Directory layout

The `--url` argument must point to a directory with this structure (following the [demo-agent output format](https://github.com/RobotOpsInc/rmw_robotops/tree/development/demo-agent#output-format)):

```
<url>/
  traces/          *.parquet  →  otel_traces view
  logs/            *.parquet  →  otel_logs view
  metrics/         *.parquet  →  otel_metrics view
  topic_messages/  *.parquet  →  topic_messages view
  mcap_metadata/   *.parquet  →  mcap_metadata view
```

Files are discovered recursively using `**/*.parquet` globs. This means each subdirectory can contain multiple Parquet files (e.g., one per hour or per recording session). Missing subdirectories are silently skipped — queries against absent tables return a `DataSourceUnavailable` error with a clear message.

## Example fixtures

The `fixtures/` subdirectory contains pre-built Parquet files generated from the SQL fixtures in `examples/duckdb/fixtures/`. They represent a simulated `robot_sim_001` running three navigation actions:

| Action | Trace | Outcome |
|--------|-------|---------|
| `/navigate_to_pose` → waypoint A | `trace-001` | Success (~8 s) |
| `/navigate_to_pose` → waypoint B | `trace-002` | Aborted (battery critical) |
| `/navigate_to_pose` → waypoint C | `trace-003` | Timed out (~30 s) |

## Regenerating fixtures

To regenerate the Parquet fixtures from the SQL sources (requires the DuckDB CLI):

```bash
# macOS
brew install duckdb

# Then run:
./examples/parquet/generate_fixtures.sh
```

The script loads all SQL fixtures into an in-memory DuckDB instance and exports each table as a Parquet file in the correct subdirectory.

## S3 usage

Point `--url` at an S3 path and set standard AWS credentials:

```bash
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_REGION=us-east-1

rosql query "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend parquet \
  --url s3://my-bucket/robot-01/robotops_demo_agent/20260403-141530/
```

S3-compatible storage (MinIO, Cloudflare R2 etc.) is also supported via `AWS_ENDPOINT_URL`:

```bash
export AWS_ENDPOINT_URL=http://localhost:9000
```

## Compile ROSQL → SQL (no --url needed)

The `compile` subcommand generates DuckDB SQL without connecting to any data source:

```bash
cargo run --features server --bin rosql -- compile \
  "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend parquet
```

## Running integration tests

```bash
cargo test --features duckdb
```

The Parquet integration tests read directly from `examples/parquet/fixtures/` — no temp databases or external services required.
