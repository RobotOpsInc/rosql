# ROSQL PostgreSQL Examples

End-to-end examples using PostgreSQL with OTel fixture data.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Rust](https://rustup.rs) (stable)
- `protoc` (`brew install protobuf` or `apt-get install protobuf-compiler`)

## Quick start

### Start PostgreSQL

```sh
just examples-up
# or: docker compose -f examples/postgres/docker-compose.yml up -d
```

**Connection string:** `postgresql://rosql:rosql@localhost:5432/rosql_examples`

### Try some queries

**Parse** (JSON AST):
```sh
cargo run --features server --bin rosql -- parse "FROM traces WHERE status = 'ERROR'"
```

**Compile to SQL** (no DB needed):
```sh
cargo run --features server --bin rosql -- compile "FROM traces WHERE status = 'ERROR'" --backend postgres
```

**Execute against the database:**
```sh
cargo run --features server,postgres --bin rosql -- query \
  "FROM traces WHERE status = 'ERROR'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples
```

### Tear down

```sh
just examples-down
# or: docker compose -f examples/postgres/docker-compose.yml down -v
```

## Example queries

### Basic

```sh
# Find all error spans
cargo run --features server,postgres --bin rosql -- query \
  "FROM traces WHERE status = 'ERROR'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples

# Filter by ROS2 node
cargo run --features server,postgres --bin rosql -- query \
  "FROM traces WHERE node = '/bt_navigator'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples
```

### Compound clauses

```sh
# Message causality graph
cargo run --features server,postgres --bin rosql -- query \
  "MESSAGE JOURNEY FOR TRACE 'trace-002'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples

# Robot health assessment
cargo run --features server,postgres --bin rosql -- query \
  "HEALTH() FOR ROBOT 'robot_sim_001'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples

# Path deviation
cargo run --features server,postgres --bin rosql -- query \
  "PATH DEVIATION FOR ROBOT 'robot_sim_001'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples

# Show MCAP recording
cargo run --features server,postgres --bin rosql -- query \
  "SHOW RECORDING" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples

# Trace all spans
cargo run --features server,postgres --bin rosql -- query \
  "TRACE 'trace-002'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples
```

## Fixture data

| File | Contents |
|------|----------|
| `fixtures/schema.sql` | OTel tables (lowercase columns, OtelPostgres profile) |
| `fixtures/traces.sql` | 11 spans, 3 nav actions, ParentSpanId causality chains |
| `fixtures/logs.sql` | 10 /rosout entries (INFO, WARN, ERROR) |
| `fixtures/metrics.sql` | CPU, memory, topic rate time series |
| `fixtures/topic_messages.sql` | /odom trajectory + /battery_state |
| `fixtures/mcap_metadata.sql` | MCAP recording session |
