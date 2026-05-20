# ROS2 OpenTelemetry Attribute Conventions

This document defines the complete schema that ROSQL expects from a ROS2 telemetry pipeline. If you're building a ROS2-to-OTel bridge or setting up your own telemetry storage, implement these conventions to get full ROSQL feature support.

## Span attributes

### ROS2 action executions

| Attribute | Type | Example | Used by ROSQL for |
|-----------|------|---------|-------------------|
| `ros.node` | string | `/bt_navigator` | `WHERE node = '...'`, `NODE_STATUS()` |
| `ros.action.name` | string | `/navigate_to_pose` | `WHERE action_name = '...'`, `ACTION_SUCCESS_RATE()` |
| `ros.action.type` | string | `nav2_msgs/action/NavigateToPose` | Action type filtering |
| `ros.action.goal_id` | string | UUID | Goal correlation |
| `ros.action.status` | string | `succeeded` / `aborted` / `canceled` | Action status filtering |

### ROS2 pub/sub tracing

| Attribute | Type | Example | Used by ROSQL for |
|-----------|------|---------|-------------------|
| `ros.topic` | string | `/cmd_vel` | Topic filtering, `MESSAGE FLOW` |
| `ros.message_type` | string | `geometry_msgs/msg/Twist` | Message type filtering |
| `ros.publisher_node` | string | `/controller_server` | Publisher attribution |
| `ros.subscriber_node` | string | `/motor_driver` | Subscriber attribution |
| `ros.plan.id` | string | UUID | `PATH DEVIATION` plan correlation (cross-repo: rmw_robotops) |

### ParentSpanId convention for TRACE

The publish span's `SpanId` must be set as the `ParentSpanId` of the corresponding subscribe span. This creates the causal chain that `TRACE` traverses.

```
Publisher span (SpanId: "abc")
  └── Subscriber span (ParentSpanId: "abc", SpanId: "def")
        └── Downstream span (ParentSpanId: "def", SpanId: "ghi")
```

Implementing this correctly requires middleware-level instrumentation — it cannot be done purely at the application level without code changes to every node.

## Metric naming conventions

TraceHouse's `MetricsCollector` emits metrics using these canonical names. ROSQL exposes them via shorthand field aliases in the query language.

### System metrics

| Metric name | Unit | Attributes | ROSQL shorthand | Description |
|-------------|------|-----------|-----------------|-------------|
| `system.cpu.utilization` | % | `cpu=all` or core index | `cpu_usage` | CPU utilization |
| `system.memory.usage` | bytes | `state=used\|available` | `memory_bytes` | Memory bytes |
| `system.memory.utilization` | % | — | `memory_usage` | Memory % |
| `system.filesystem.usage` | bytes | `mountpoint`, `state=used\|free` | `disk_bytes` | Disk bytes |
| `system.filesystem.utilization` | % | `mountpoint` | `disk_usage` | Disk % per mountpoint |
| `system.disk.io` | bytes/s | `device`, `direction=read\|write` | `disk_io` | Disk throughput |
| `system.disk.operations` | ops/s | `device` | `disk_iops` | Disk IOPS |
| `system.network.io` | bytes/s | `device`, `direction=sent\|recv` | `network_io` | Network throughput |
| `system.network.packets` | packets/s | `device` | `network_packets` | Network packet rate |
| `system.network.latency` | ms | `stat=avg\|min\|max` | `network_latency` | Ping latency |
| `system.network.jitter` | ms | — | `network_jitter` | Network jitter |
| `system.network.packet_loss` | % | — | `packet_loss` | Packet loss rate |
| `system.temperature` | °C | `sensor.id`, `sensor.type` | `temperature` | Sensor temperature |
| `system.battery.charge` | % | `battery.id` | `battery_charge` | Battery charge level |
| `system.battery.voltage` | V | `battery.id` | `battery_voltage` | Battery voltage |
| `system.battery.current` | A | `battery.id` | `battery_current` | Battery current draw |
| `system.battery.temperature` | °C | `battery.id` | `battery_temperature` | Battery temperature |

### ROS2 runtime metrics

| Metric name | Unit | Attributes | ROSQL shorthand | Description |
|-------------|------|-----------|-----------------|-------------|
| `ros2.topic.message_rate` | Hz | `topic.name`, `topic.type` | `publish_rate` | Topic message rate |
| `ros2.topic.bandwidth` | bytes/s | `topic.name` | `bandwidth` | Topic bandwidth |
| `ros2.topic.messages_received` | count | `topic.name` | `messages_received` | Total messages received |
| `ros2.topic.messages_captured` | count | `topic.name` | `messages_captured` | Total messages captured |
| `ros2.topic.messages_filtered` | count | `topic.name` | `messages_filtered` | Total messages filtered |
| `ros2.action_servers.count` | count | — | `action_servers_count` | Action servers discovered |
| `ros2.services.count` | count | — | `services_count` | Services discovered |
| `ros2.action.queued_goals` | count | `action.name` | `queued_goals` | Per-action queued goals |
| `ros2.action.active_goals` | count | `action.name` | `active_goals` | Per-action active goals |
| `ros2.action.completion_rate` | Hz | `action.name` | `completion_rate` | Per-action completion rate |

### Process metrics

| Metric name | Unit | Attributes | ROSQL shorthand | Description |
|-------------|------|-----------|-----------------|-------------|
| `process.cpu.utilization` | % | `process.pid`, `process.name` | `process_cpu` | Per-process CPU |
| `process.memory.usage` | bytes | `process.pid`, `type=rss\|vms` | `process_memory` | Per-process memory |

### Deprecated metric names

These legacy names are emitted by older agent versions. New deployments should use the canonical names above.

| Legacy name | Replaced by |
|-------------|-------------|
| `system.cpu.total_usage_pct` | `system.cpu.utilization` |
| `system.memory.usage_pct` | `system.memory.utilization` |
| `ros2.topic.rx_rate_hz` | `ros2.topic.message_rate` |
| `ros2.topic.rx_bandwidth_bps` | `ros2.topic.bandwidth` |
| `ros2.topic.last_message_age_ms` | _(no longer collected by default)_ |

## Resource attributes

Resource attributes are set at the bridge/SDK level and enable `FOR` clause scoping across all query types.

| Attribute | Type | Example | Used by ROSQL for |
|-----------|------|---------|-------------------|
| `robot.id` | string | `robot_42` | `FOR ROBOT` scoping |
| `service.version` | string | `2.3.1` | `FOR VERSION` scoping |
| `deployment.environment` | string | `production` | `FOR ENVIRONMENT` scoping |
| `ros.session.id` | string | `delivery_042` | `FOR SESSION` scoping |
| `ros.session.type` | string | `delivery` | WHERE filtering |
| `ros.plan.id` | string | UUID | `PATH DEVIATION` plan correlation |

## Complete table definitions

These are the exact tables ROSQL queries against. The DDL below uses PostgreSQL syntax with the `otel-postgres` schema profile (lowercase column names). For the `otel-clickhouse` profile, column names are PascalCase (e.g. `TraceId`, `StatusCode`).

### Required tables

These three tables are required for basic ROSQL functionality.

#### `otel_traces`

```sql
CREATE TABLE otel_traces (
    timestamp            TIMESTAMPTZ NOT NULL,
    trace_id             TEXT NOT NULL,
    span_id              TEXT NOT NULL,
    parent_span_id       TEXT NOT NULL DEFAULT '',
    span_name        TEXT NOT NULL,
    span_kind            TEXT NOT NULL DEFAULT 'INTERNAL',
    service_name         TEXT NOT NULL DEFAULT '',
    duration             BIGINT NOT NULL,          -- nanoseconds
    status_code          TEXT NOT NULL DEFAULT 'OK',
    span_attributes      JSONB NOT NULL DEFAULT '{}',
    resource_attributes  JSONB NOT NULL DEFAULT '{}'
);
```

Used by: `FROM traces`, `TRACE`, `HEALTH()`, `ANOMALY()`, `DURING()`, `CORRELATE`, `SINCE last action failure`

#### `otel_logs`

```sql
CREATE TABLE otel_logs (
    timestamp            TIMESTAMPTZ NOT NULL,
    trace_id             TEXT NOT NULL DEFAULT '',
    span_id              TEXT NOT NULL DEFAULT '',
    severity_text        TEXT NOT NULL DEFAULT 'INFO',
    severity_number      INTEGER NOT NULL DEFAULT 9,
    service_name         TEXT NOT NULL DEFAULT '',
    body                 TEXT NOT NULL DEFAULT '',
    resource_attributes  JSONB NOT NULL DEFAULT '{}',
    log_attributes       JSONB NOT NULL DEFAULT '{}'
);
```

Used by: `FROM logs`, `HEALTH()`

#### `otel_metrics`

```sql
CREATE TABLE otel_metrics (
    timestamp            TIMESTAMPTZ NOT NULL,
    metric_name          TEXT NOT NULL,
    value                DOUBLE PRECISION NOT NULL,
    attributes           JSONB NOT NULL DEFAULT '{}',
    service_name         TEXT NOT NULL DEFAULT ''
);
```

Used by: `FROM metrics`, `HEALTH()`, `CORRELATE`

### Optional tables

These tables enable additional ROSQL features. If absent, ROSQL returns a clear `DataSourceUnavailable` error with guidance.

#### `topic_messages`

```sql
CREATE TABLE topic_messages (
    robot_id             TEXT NOT NULL,
    topic_name           TEXT NOT NULL,
    timestamp            TIMESTAMPTZ NOT NULL,
    fields               JSONB NOT NULL DEFAULT '{}',
    message_type         TEXT NOT NULL DEFAULT ''
);
```

Used by: `FROM topics`, `FROM odom` (and other topic aliases), `PATH DEVIATION`, `JOINT DEVIATION`, `WITHIN`

#### `mcap_metadata`

```sql
CREATE TABLE mcap_metadata (
    robot_id             TEXT NOT NULL,
    session_id           TEXT NOT NULL,
    start_time           TIMESTAMPTZ NOT NULL,
    end_time             TIMESTAMPTZ NOT NULL,
    file_uri             TEXT NOT NULL,
    topics               TEXT[] NOT NULL DEFAULT '{}',
    message_types        JSONB NOT NULL DEFAULT '{}'  -- topic → message_type map
);
```

`file_uri` is a full URI pointing to the MCAP file. Supports `s3://`, `file://`, and `gs://` schemes (e.g. `s3://bucket/path/session.mcap`, `file:///var/ros/recordings/session.mcap`).

The `message_types` column maps topic names to their ROS2 message types:

```json
{
  "/camera/image_raw": "sensor_msgs/Image",
  "/cmd_vel": "geometry_msgs/msg/Twist",
  "/odom": "nav_msgs/Odometry"
}
```

Used by: `FROM recordings`, `FROM recordings WHERE topic = '...'`

#### `robot_joint_map` _(v0.4.3+, optional)_

```sql
CREATE TABLE robot_joint_map (
    robot_model          TEXT NOT NULL,
    valid_from           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_to             TIMESTAMPTZ,
    version              TEXT NOT NULL DEFAULT '',
    robot_ids            TEXT[] NOT NULL DEFAULT '{}',
    joint_map            JSONB NOT NULL DEFAULT '[]'
);
```

The `joint_map` column is a JSON array of joint descriptors:

```json
[
  { "name": "shoulder_pan", "index": 0, "type": "revolute", "lower_limit": -3.14, "upper_limit": 3.14 },
  { "name": "shoulder_lift", "index": 1, "type": "revolute", "lower_limit": -1.57, "upper_limit": 1.57 }
]
```

Used by: `SHOW JOINTS`, `JOINT DEVIATION`

## ROSQL field mappings

These are the ROSQL field names and what columns they resolve to.

### From `otel_traces`

| ROSQL field | Column | Storage unit |
|-------------|--------|-------------|
| `trace_id` | `trace_id` | — |
| `span_id` | `span_id` | — |
| `parent_span_id` | `parent_span_id` | — |
| `span_name` | `span_name` | — |
| `service` | `service_name` | — |
| `duration` | `duration` | nanoseconds |
| `status` | `status_code` | OK / ERROR |
| `node` | `span_attributes->>'ros.node'` | — |
| `action_name` | `span_attributes->>'ros.action.name'` | — |
| `action_status` | `span_attributes->>'ros.action.status'` | — |
| `topic` | `span_attributes->>'ros.topic'` | — |
| `robot_id` | `resource_attributes->>'robot.id'` | — |
| `org_id` | `resource_attributes->>'organization.id'` | — |

### From `otel_metrics`

| ROSQL field | Metric name | Unit |
|-------------|-------------|------|
| `publish_rate` | `ros2.topic.message_rate` | Hz |
| `bandwidth` | `ros2.topic.bandwidth` | B/s |
| `messages_received` | `ros2.topic.messages_received` | count |
| `messages_captured` | `ros2.topic.messages_captured` | count |
| `messages_filtered` | `ros2.topic.messages_filtered` | count |
| `action_servers_count` | `ros2.action_servers.count` | count |
| `services_count` | `ros2.services.count` | count |
| `queued_goals` | `ros2.action.queued_goals` | count |
| `active_goals` | `ros2.action.active_goals` | count |
| `completion_rate` | `ros2.action.completion_rate` | Hz |
| `cpu_usage` | `system.cpu.utilization` | % |
| `memory_usage` | `system.memory.utilization` | % |
| `memory_bytes` | `system.memory.usage` | bytes |
| `disk_usage` | `system.filesystem.utilization` | % |
| `disk_bytes` | `system.filesystem.usage` | bytes |
| `disk_io` | `system.disk.io` | B/s |
| `disk_iops` | `system.disk.operations` | ops/s |
| `network_io` | `system.network.io` | B/s |
| `network_packets` | `system.network.packets` | packets/s |
| `network_latency` | `system.network.latency` | ms |
| `network_jitter` | `system.network.jitter` | ms |
| `packet_loss` | `system.network.packet_loss` | % |
| `temperature` | `system.temperature` | °C |
| `battery_charge` | `system.battery.charge` | % |
| `battery_voltage` | `system.battery.voltage` | V |
| `battery_current` | `system.battery.current` | A |
| `battery_temperature` | `system.battery.temperature` | °C |
| `process_cpu` | `process.cpu.utilization` | % |
| `process_memory` | `process.memory.usage` | bytes |

### From `otel_logs`

| ROSQL field | Column |
|-------------|--------|
| `message` | `body` |
| `severity` | `severity_text` |
| `severity_number` | `severity_number` |
| `robot_id` | `resource_attributes->>'robot.id'` |
| `org_id` | `resource_attributes->>'organization.id'` |

### From `topic_messages` (position and joint data)

ROSQL `WITHIN` and `JOINT DEVIATION` extract values from the `fields` JSONB column using these paths:

| ROSQL path | `fields` JSON path | ROS2 message type | Notes |
|---|---|---|---|
| `position.latitude` | `fields->>'position.latitude'` | `sensor_msgs/NavSatFix` | GPS lat |
| `position.longitude` | `fields->>'position.longitude'` | `sensor_msgs/NavSatFix` | GPS lon |
| `gps.lat` | `fields->>'position.latitude'` | `sensor_msgs/NavSatFix` | Alias |
| `gps.lon` | `fields->>'position.longitude'` | `sensor_msgs/NavSatFix` | Alias |
| `pose.pose.position.x` | `fields->>'pose.pose.position.x'` | `nav_msgs/Odometry` | Local x |
| `pose.pose.position.y` | `fields->>'pose.pose.position.y'` | `nav_msgs/Odometry` | Local y |
| `position.x` | `fields->>'pose.pose.position.x'` | `nav_msgs/Odometry` | Alias |
| `position.y` | `fields->>'pose.pose.position.y'` | `nav_msgs/Odometry` | Alias |
| `orientation.yaw` | computed from quaternion | `nav_msgs/Odometry` | Computed via atan2 |
| `position[N]` | `fields->'position'->>N` | `sensor_msgs/JointState` | Joint position at index N |
| `velocity[N]` | `fields->'velocity'->>N` | `sensor_msgs/JointState` | Joint velocity at index N |
| `effort[N]` | `fields->'effort'->>N` | `sensor_msgs/JointState` | Joint effort at index N |

The `orientation.yaw` field is computed as `atan2(2*(qw*qz + qx*qy), 1 - 2*(qy^2 + qz^2))` from the quaternion components in `nav_msgs/Odometry`.

## Schema profiles

Different OTel Collector exporters use different column naming conventions. ROSQL supports multiple profiles:

| Profile | Convention | Example | Default for |
|---------|-----------|---------|-------------|
| `otel-postgres` | Lowercase | `trace_id`, `status_code` | PostgreSQL, MySQL |
| `otel-clickhouse` | PascalCase | `TraceId`, `StatusCode` | ClickHouse |

Select the profile with the `--schema` CLI flag:

```sh
rosql compile "FROM traces WHERE status = 'ERROR'" --backend postgres --schema otel-postgres
```

Custom schema profiles (user-defined column mappings) are tracked in [#22](https://github.com/RobotOpsInc/rosql/issues/22).

## Topic aliases

These ROSQL source names are aliases for common ROS2 topics:

| ROSQL source | Resolves to |
|-------------|------------|
| `FROM odom` | `FROM topics WHERE topic_name = '/odom'` |
| `FROM joint_states` | `FROM topics WHERE topic_name = '/joint_states'` |
| `FROM battery` | `FROM topics WHERE topic_name = '/battery_state'` |
| `FROM cmd_vel` | `FROM topics WHERE topic_name = '/cmd_vel'` |
| `FROM imu` | `FROM topics WHERE topic_name = '/imu/data'` |
