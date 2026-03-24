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
- [just](https://just.systems) (optional — or run the commands manually)
- `protoc` (`brew install protobuf` or `apt-get install protobuf-compiler`)

### Start PostgreSQL with fixture data

```sh
cd examples
docker compose up -d
```

This starts PostgreSQL on `localhost:5432` and automatically loads the OTel schema + fixture data.

**Connection string:** `postgresql://rosql:rosql@localhost:5432/rosql_examples`

### Run example queries

From the repo root:

```sh
# Parse all example queries (shows JSON AST output)
just run-examples

# Or run individual queries:
cargo build --features server --bin rosql-parser
./target/debug/rosql-parser parse "FROM traces WHERE status = 'ERROR'"
```

### Tear down

```sh
cd examples
docker compose down -v
```

## Example queries

### Basic queries (`queries/basic.rosql`)

```sql
-- Find all error spans
FROM traces WHERE status = 'ERROR'

-- Filter by ROS2 node name
FROM traces WHERE node = '/bt_navigator'

-- Use a topic alias
FROM odom LIMIT 5
```

### Compound clauses (`queries/compound_clauses.rosql`)

These are the "killer demo" queries that showcase ROSQL's unique features:

```sql
-- Which navigation action failed?
FROM traces WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'

-- Trace the full message causality chain
MESSAGE JOURNEY FOR TRACE 'trace-002'

-- Robot health assessment across all signal types
HEALTH() FOR ROBOT 'robot_sim_001'

-- Did the robot deviate from its planned path?
PATH DEVIATION FOR ROBOT 'robot_sim_001'

-- Find the MCAP recording covering the failure
SHOW RECORDING

-- Detect anomalous span durations
ANOMALY(duration)
```

### Pipeline syntax (`queries/pipeline.rosql`)

```sql
-- Chain stages with | for readable composition
FROM traces
| WHERE duration > 500 ms
| WHERE status = 'ERROR'
| FACET robot_id
```

### Aggregations (`queries/timeseries.rosql`)

```sql
-- Average and max duration with aliases
SELECT AVG(duration) AS avg_dur, MAX(duration) AS max_dur, COUNT(*) AS total FROM traces

-- 95th percentile navigation duration
SELECT PERCENTILE(duration, 95) FROM traces WHERE action_name = '/navigate_to_pose'
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
3. Point ROSQL at your database: `rosql-parser parse "FROM traces WHERE node = '/your_node'"`

See the [ROS2 OTel attribute conventions](../docs/ros2-otel-conventions.md) for the expected span attributes.
