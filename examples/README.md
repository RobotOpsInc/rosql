# ROSQL Examples

End-to-end examples with realistic ROS2 telemetry fixture data.

## What the fixture data represents

A simulated ROS2 robot (`robot_sim_001`) running Nav2 performs three navigation actions over ~10 minutes:

| Action | Goal | Result | Duration | What happens |
|--------|------|--------|----------|-------------|
| 1 | Waypoint A (5, 3) | Success | ~8s | Clean navigation, smooth path |
| 2 | Waypoint B (10, 7) | **Aborted** | ~12s | Battery drops to 15%, CPU spikes to 92%, path deviates |
| 3 | Waypoint C (15, 2) | **Timed out** | ~30s | Controller makes no progress |

All signals are interconnected:
- **Traces** have `ParentSpanId` chains: root → bt_navigator → controller → costmap
- **Logs** show warnings and errors correlating with the failures
- **Metrics** show CPU spike and topic rate drop during the abort
- **/odom** trajectory shows the robot veering off course during action 2
- **/battery_state** percentage drops below 20% during action 2
- An **MCAP recording** covers the entire session

## Quick start

### Prerequisites

- [Rust](https://rustup.rs) (stable)
- [Docker](https://docs.docker.com/get-docker/) (for PostgreSQL)
- `protoc` (`brew install protobuf` or `apt-get install protobuf-compiler`)

### Start PostgreSQL with fixture data

```sh
just examples-up
# or: docker compose -f examples/docker-compose.yml up -d
```

This starts PostgreSQL on `localhost:5432` and automatically loads the OTel schema + fixture data.

### Try some queries

**Parse a query** (shows the AST as JSON):

```sh
cargo run --features server --bin rosql-parser -- parse "FROM traces WHERE status = 'ERROR'"
```

**Compile a query to SQL** (shows what SQL ROSQL generates — no DB needed):

```sh
cargo run --features server --bin rosql-parser -- compile "FROM traces WHERE status = 'ERROR'" --backend postgres
```

**Execute a query against the example database** (returns actual results):

```sh
cargo run --features server,sql --bin rosql-parser -- query \
  "FROM traces WHERE status = 'ERROR'" \
  --backend postgres --url postgresql://rosql:rosql@localhost:5432/rosql_examples
```

### Tear down

```sh
just examples-down
# or: docker compose -f examples/docker-compose.yml down -v
```

## Example queries

### Basic queries

```sh
# Find all error spans
cargo run --features server --bin rosql-parser -- compile \
  "FROM traces WHERE status = 'ERROR'" --backend postgres

# Filter by ROS2 node
cargo run --features server --bin rosql-parser -- compile \
  "FROM traces WHERE node = '/bt_navigator'" --backend postgres

# Topic alias
cargo run --features server --bin rosql-parser -- compile \
  "FROM odom LIMIT 5" --backend postgres

# Unit conversion (500 ms → nanoseconds in compiled SQL)
cargo run --features server --bin rosql-parser -- compile \
  "FROM traces WHERE duration > 500 ms" --backend postgres
```

### Compound clauses — the killer demos

```sh
# Message causality graph — trace how a message propagated through nodes
cargo run --features server --bin rosql-parser -- compile \
  "MESSAGE JOURNEY FOR TRACE 'trace-002'" --backend postgres

# Robot health assessment across traces, logs, and metrics
cargo run --features server --bin rosql-parser -- compile \
  "HEALTH() FOR ROBOT 'robot_sim_001'" --backend postgres

# Path deviation analysis — did the robot veer off course?
cargo run --features server --bin rosql-parser -- compile \
  "PATH DEVIATION FOR ROBOT 'robot_sim_001'" --backend postgres

# Find the MCAP recording covering the failure
cargo run --features server --bin rosql-parser -- compile \
  "SHOW RECORDING" --backend postgres

# Statistical anomaly detection
cargo run --features server --bin rosql-parser -- compile \
  "ANOMALY(duration)" --backend postgres

# Show all spans for a trace
cargo run --features server --bin rosql-parser -- compile \
  "TRACE 'trace-002'" --backend postgres
```

### Pipeline syntax

```sh
cargo run --features server --bin rosql-parser -- compile \
  "FROM traces | WHERE duration > 500 ms | WHERE status = 'ERROR' | FACET robot_id" \
  --backend postgres
```

### Compare SQL dialects

```sh
# PostgreSQL
cargo run --features server --bin rosql-parser -- compile \
  "FROM traces WHERE node = '/bt_navigator'" --backend postgres

# SQLite
cargo run --features server --bin rosql-parser -- compile \
  "FROM traces WHERE node = '/bt_navigator'" --backend sqlite

# MySQL
cargo run --features server --bin rosql-parser -- compile \
  "FROM traces WHERE node = '/bt_navigator'" --backend mysql
```

## Running tests

Example queries are automatically tested in CI via `cargo test`. The integration tests parse and compile every query in the `.rosql` files.

For full end-to-end testing against PostgreSQL with fixture data:

```sh
just test-examples
```

## Fixture data files

| File | Contents |
|------|----------|
| `fixtures/schema.sql` | OTel table definitions (otel_traces, otel_logs, otel_metrics, topic_messages, mcap_metadata) |
| `fixtures/traces.sql` | 11 spans across 3 navigation actions with ParentSpanId causality chains |
| `fixtures/logs.sql` | 10 /rosout log entries (INFO, WARN, ERROR) |
| `fixtures/metrics.sql` | Topic rates, CPU usage, memory usage time series |
| `fixtures/topic_messages.sql` | /odom trajectory + /battery_state readings |
| `fixtures/mcap_metadata.sql` | One MCAP recording session covering the full timeline |

## Adapting to your own data

To use ROSQL with your own ROS2 telemetry:

1. Set up an [OTel Collector](https://opentelemetry.io/docs/collector/) exporting to PostgreSQL
2. Ensure your ROS2 nodes emit spans with `ros.node`, `ros.action.name`, and `ParentSpanId` attributes
3. Run queries:

```sh
cargo run --features server,sql --bin rosql-parser -- query \
  "FROM traces WHERE node = '/your_node'" \
  --backend postgres --url postgresql://user:pass@host:5432/your_db
```

See the [ROS2 OTel attribute conventions](../docs/ros2-otel-conventions.md) for the expected span attributes.
