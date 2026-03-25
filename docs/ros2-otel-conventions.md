# ROS2 OpenTelemetry Attribute Conventions

This document defines the OTel span attributes and metric names that ROSQL expects from a ROS2 telemetry pipeline. If you're building a ROS2-to-OTel bridge, implement these conventions to get full ROSQL feature support.

## Span attributes for ROS2 action executions

| Attribute | Type | Example | Used by ROSQL for |
|-----------|------|---------|-------------------|
| `ros.node` | string | `/bt_navigator` | `WHERE node = '...'`, `NODE_STATUS()` |
| `ros.action.name` | string | `/navigate_to_pose` | `WHERE action_name = '...'`, `ACTION_SUCCESS_RATE()` |
| `ros.action.type` | string | `nav2_msgs/action/NavigateToPose` | Action type filtering |
| `ros.action.goal_id` | string | UUID | Goal correlation |
| `ros.action.status` | string | `succeeded` / `aborted` / `canceled` | Action status filtering |
| `ros.topic` | string | `/cmd_vel` | Topic span attribution |

## Span attributes for ROS2 pub/sub tracing

| Attribute | Type | Example | Used by ROSQL for |
|-----------|------|---------|-------------------|
| `ros.topic` | string | `/cmd_vel` | Topic filtering |
| `ros.message_type` | string | `geometry_msgs/msg/Twist` | Message type filtering |
| `ros.publisher_node` | string | `/controller_server` | Publisher attribution |
| `ros.subscriber_node` | string | `/motor_driver` | Subscriber attribution |

## ParentSpanId convention for MESSAGE JOURNEY

The publish span's `SpanId` must be set as the `ParentSpanId` of the corresponding subscribe span. This creates the causal chain that `MESSAGE JOURNEY` traverses.

```
Publisher span (SpanId: "abc")
  └── Subscriber span (ParentSpanId: "abc", SpanId: "def")
        └── Downstream span (ParentSpanId: "def", SpanId: "ghi")
```

Implementing this correctly requires middleware-level instrumentation — it cannot be done purely at the application level without code changes to every node.

## Metric naming conventions

| Metric name | Unit | Description |
|-------------|------|-------------|
| `ros2.topic.rx_rate_hz` | Hz | Topic receive rate |
| `ros2.topic.rx_bandwidth_bps` | B/s | Topic bandwidth |
| `ros2.topic.last_message_age_ms` | ms | Age of last received message |
| `system.cpu.total_usage_pct` | % | Total CPU usage |
| `system.memory.usage_pct` | % | Memory usage |

## Required OTel tables

ROSQL expects telemetry stored in standard OTel Collector output tables:

| Table | Required columns | Used by |
|-------|-----------------|---------|
| `otel_traces` | trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, timestamp | `FROM traces`, `MESSAGE JOURNEY`, `TRACE` |
| `otel_logs` | timestamp, body, severity_text, severity_number, service_name | `FROM logs` |
| `otel_metrics` | timestamp, metric_name, value, attributes, service_name | `FROM metrics` |

### Optional tables (enable additional features)

| Table | Required columns | Enables |
|-------|-----------------|---------|
| `topic_messages` | robot_id, topic_name, timestamp, fields, message_type | `FROM topics`, `PATH DEVIATION`, topic aliases |
| `mcap_metadata` | robot_id, session_id, start_time, end_time, s3_key, topics | `SHOW RECORDING`, `FROM recordings` |

If optional tables are absent, ROSQL returns a clear `DataSourceUnavailable` error with guidance on how to configure the missing data source.

## Schema profiles

Column naming varies by OTel Collector exporter:

| Profile | Convention | Example columns | Used by |
|---------|-----------|-----------------|---------|
| `otel-postgres` | Lowercase | `trace_id`, `status_code`, `span_attributes` | OTel Collector PostgreSQL exporter |
| `otel-clickhouse` | PascalCase | `TraceId`, `StatusCode`, `SpanAttributes` | OTel Collector ClickHouse exporter |

Select the profile with the `--schema` flag:

```sh
rosql compile "FROM traces WHERE status = 'ERROR'" --backend postgres --schema otel-postgres
```
