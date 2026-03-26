# ROSQL × DuckDB Example

Run ROSQL queries against a DuckDB embedded database. No Docker or external server required — DuckDB runs in-process.

## Quick start

```bash
# Build with DuckDB support
cargo build --features server,duckdb --bin rosql

# Create the example database from fixtures
duckdb examples/duckdb/rosql_examples.db < examples/duckdb/fixtures/01_schema.sql
duckdb examples/duckdb/rosql_examples.db < examples/duckdb/fixtures/02_traces.sql
duckdb examples/duckdb/rosql_examples.db < examples/duckdb/fixtures/03_logs.sql
duckdb examples/duckdb/rosql_examples.db < examples/duckdb/fixtures/04_metrics.sql
duckdb examples/duckdb/rosql_examples.db < examples/duckdb/fixtures/05_topic_messages.sql
duckdb examples/duckdb/rosql_examples.db < examples/duckdb/fixtures/06_mcap_metadata.sql

# Run example queries
cargo run --features server,duckdb --bin rosql -- query \
  "FROM traces WHERE status = 'ERROR'" \
  --backend duckdb --url duckdb:///$(pwd)/examples/duckdb/rosql_examples.db

cargo run --features server,duckdb --bin rosql -- query \
  "MESSAGE JOURNEY FOR TRACE 'trace-002'" \
  --backend duckdb --url duckdb:///$(pwd)/examples/duckdb/rosql_examples.db

cargo run --features server,duckdb --bin rosql -- query \
  "HEALTH()" \
  --backend duckdb --url duckdb:///$(pwd)/examples/duckdb/rosql_examples.db
```

## Connection strings

| Format | Description |
|--------|-------------|
| `duckdb://` | In-memory database (data lost on disconnect) |
| `duckdb:///path/to/file.db` | Persistent file-based database |
| `duckdb:///path/to/file.duckdb` | Same — DuckDB accepts any extension |

## Compile ROSQL → DuckDB SQL

The `compile` subcommand shows the SQL ROSQL generates for DuckDB:

```bash
cargo run --features server --bin rosql -- compile \
  "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \
  --backend duckdb
```

Output includes `NOW()::TIMESTAMP` for interval arithmetic, which is DuckDB's required form.

## Schema

The fixtures use the standard OTel tables with DuckDB-compatible types:
- `JSONB` → `JSON` (DuckDB does not have a JSONB type)
- `TEXT[]` → `VARCHAR[]`

See [`fixtures/01_schema.sql`](fixtures/01_schema.sql) for the complete DDL.

## Running integration tests

```bash
cargo test --features duckdb
```

The DuckDB integration tests run without Docker — they use an in-memory database loaded from the fixture files.
