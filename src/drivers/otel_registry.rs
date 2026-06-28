//! OTel schema profiles — maps ROSQL fields to database column names.
//!
//! Different OTel Collector exporters use different column naming conventions.
//! Each schema profile maps ROSQL field names to the actual column names
//! for that exporter's output.

use super::field_registry::{FieldDef, FieldRegistry};
use serde::{Deserialize, Serialize};

/// Built-in schema profiles for common OTel Collector exporters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaProfile {
    /// Lowercase columns — used by the OTel Collector PostgreSQL exporter.
    /// Example: trace_id, span_name, status_code, span_attributes
    OtelPostgres,
    /// PascalCase columns — used by the OTel Collector ClickHouse exporter.
    /// Example: TraceId, SpanName, StatusCode, SpanAttributes
    OtelClickhouse,
}

impl SchemaProfile {
    /// Column name mappings for this profile.
    fn col(&self, postgres_name: &'static str, clickhouse_name: &'static str) -> &'static str {
        match self {
            SchemaProfile::OtelPostgres => postgres_name,
            SchemaProfile::OtelClickhouse => clickhouse_name,
        }
    }
}

/// Build a field registry for the given schema profile.
pub fn otel_registry(profile: SchemaProfile) -> FieldRegistry {
    let mut reg = FieldRegistry::new();

    // ── Table name mappings ─────────────────────────────────────────
    reg.register_table("logs", "otel_logs");
    reg.register_table("traces", "otel_traces");
    reg.register_table("metrics", "otel_metrics");
    reg.register_table("diagnostics", "otel_metrics");
    reg.register_table("topics", "topic_messages");
    reg.register_table("recordings", "mcap_metadata");
    reg.register_table("tf", "tf_states");
    reg.register_table("heartbeats", "robot_heartbeats");
    reg.register_table("system_logs", "system_logs");
    reg.register_table("events", "ros2_events");
    reg.register_table("node_graph", "node_graph_edges");
    reg.register_table("joints", "joint_states");

    // ── otel_traces fields ──────────────────────────────────────────
    let trace_id = profile.col("trace_id", "TraceId");
    let span_id = profile.col("span_id", "SpanId");
    let parent_span_id = profile.col("parent_span_id", "ParentSpanId");
    let span_name = profile.col("span_name", "SpanName");
    let service_name = profile.col("service_name", "ServiceName");
    let duration = profile.col("duration", "Duration");
    let status_code = profile.col("status_code", "StatusCode");
    let span_attrs = profile.col("span_attributes", "SpanAttributes");
    let timestamp = profile.col("timestamp", "Timestamp");

    reg.register(simple("trace_id", "otel_traces", trace_id));
    reg.register(simple("span_id", "otel_traces", span_id));
    reg.register(simple("parent_span_id", "otel_traces", parent_span_id));
    reg.register(simple("span_name", "otel_traces", span_name));
    reg.register(simple("service", "otel_traces", service_name));
    reg.register(FieldDef {
        name: "duration".into(),
        source_table: "otel_traces".into(),
        column: duration.into(),
        storage_unit: Some("ns".into()),
        ..Default::default()
    });
    reg.register(simple("status", "otel_traces", status_code));
    reg.register(simple("timestamp", "otel_traces", timestamp));

    // ROS2 span attributes (map access)
    reg.register(map_field("node", "otel_traces", span_attrs, "ros.node"));
    reg.register(map_field(
        "action_name",
        "otel_traces",
        span_attrs,
        "ros.action.name",
    ));
    reg.register(map_field(
        "action_status",
        "otel_traces",
        span_attrs,
        "ros.action.status",
    ));
    reg.register(map_field("topic", "otel_traces", span_attrs, "ros.topic"));
    // ROS pub/sub attribution + message type (used by SHOW TOPICS/NODES/NODE GRAPH
    // and MESSAGE FLOW). Registered so the SHOW/flow compilers resolve their
    // attribute keys through the registry rather than hardcoding `ros.*` literals.
    reg.register(map_field(
        "message_type",
        "otel_traces",
        span_attrs,
        "ros.message_type",
    ));
    reg.register(map_field(
        "publisher_node",
        "otel_traces",
        span_attrs,
        "ros.publisher_node",
    ));
    reg.register(map_field(
        "subscriber_node",
        "otel_traces",
        span_attrs,
        "ros.subscriber_node",
    ));

    // ── robot.* concept vocabulary (ROB-432) ────────────────────────────
    // Portable, robotics-general concept keys per the robotics semantic
    // conventions v0 (`robot.*`). These let queries filter/select on the
    // durable concept vocabulary directly (e.g. `SELECT robot.action.result`,
    // `WHERE robot.component = '...'`) regardless of transport. The field name
    // is the dotted concept key itself; it resolves to the same span_attributes
    // map access. ROS-specific keys (`ros.*`) remain registered above unchanged.
    for key in [
        "robot.action.name",
        "robot.action.goal_id",
        "robot.action.status",
        "robot.action.result",
        "robot.component",
        "robot.transform.parent",
        "robot.transform.child",
        "robot.joint.name",
        "robot.trajectory.point_count",
        "robot.target.frame",
        "robot.object.id",
    ] {
        reg.register(map_field(key, "otel_traces", span_attrs, key));
    }

    // ── Generic concept aliases (ROB-432) ───────────────────────────────
    // Transport-neutral field names that prefer the portable `robot.*` key and
    // transparently fall back to the `ros.*` mapping for ROS data (via COALESCE
    // at compile time). The existing ROS field names (`node`, `topic`, …) keep
    // working unchanged — these are additive.
    //   component → robot.component  (fallback: ros.node)
    //   action    → robot.action.name (fallback: ros.action.name)
    //   channel   → ros.topic         (no portable `robot.channel.*` exists yet)
    reg.register(map_field_fallback(
        "component",
        "otel_traces",
        span_attrs,
        "robot.component",
        &["ros.node"],
    ));
    reg.register(map_field_fallback(
        "action",
        "otel_traces",
        span_attrs,
        "robot.action.name",
        &["ros.action.name"],
    ));
    reg.register(map_field("channel", "otel_traces", span_attrs, "ros.topic"));

    // Resource attributes shared across all OTel tables.
    // robot_id and org_id live in resource_attributes on Postgres/DuckDB.
    // The Clickhouse dialect (#98) will override these to use the materialized
    // RobotId and OrgId columns with bloom filter indexes.
    // On topic_messages, robot_id is a bare column (registered separately below).
    let res_attrs = "resource_attributes";
    for otel_table in ["otel_traces", "otel_logs", "otel_metrics"] {
        reg.register(map_field("robot_id", otel_table, res_attrs, "robot.id"));
        reg.register(map_field(
            "org_id",
            otel_table,
            res_attrs,
            "organization.id",
        ));
    }

    // ── otel_metrics fields ─────────────────────────────────────────
    let metric_name = profile.col("metric_name", "MetricName");
    let metric_value = profile.col("value", "Value");

    reg.register(simple("metric_name", "otel_metrics", metric_name));
    reg.register(simple("metric_value", "otel_metrics", metric_value));

    // ── ROS2 topic metrics ──────────────────────────────────────────
    // Canonical names (robot_agent MetricsCollector output).
    reg.register(metric_field(
        "publish_rate",
        "ros2.topic.message_rate",
        Some("Hz"),
        metric_value,
    ));
    reg.register(metric_field(
        "bandwidth",
        "ros2.topic.bandwidth",
        Some("B/s"),
        metric_value,
    ));
    reg.register(metric_field(
        "messages_received",
        "ros2.topic.messages_received",
        None,
        metric_value,
    ));
    reg.register(metric_field(
        "messages_captured",
        "ros2.topic.messages_captured",
        None,
        metric_value,
    ));
    reg.register(metric_field(
        "messages_filtered",
        "ros2.topic.messages_filtered",
        None,
        metric_value,
    ));
    reg.register(metric_field(
        "action_servers_count",
        "ros2.action_servers.count",
        None,
        metric_value,
    ));
    reg.register(metric_field(
        "services_count",
        "ros2.services.count",
        None,
        metric_value,
    ));
    reg.register(metric_field(
        "queued_goals",
        "ros2.action.queued_goals",
        None,
        metric_value,
    ));
    reg.register(metric_field(
        "active_goals",
        "ros2.action.active_goals",
        None,
        metric_value,
    ));
    reg.register(metric_field(
        "completion_rate",
        "ros2.action.completion_rate",
        Some("Hz"),
        metric_value,
    ));

    // ── System metrics ──────────────────────────────────────────────
    // Canonical names (OTel semantic conventions, robot_agent output).
    reg.register(metric_field(
        "cpu_usage",
        "system.cpu.utilization",
        Some("%"),
        metric_value,
    ));
    reg.register(metric_field(
        "memory_usage",
        "system.memory.utilization",
        Some("%"),
        metric_value,
    ));
    reg.register(metric_field(
        "memory_bytes",
        "system.memory.usage",
        Some("B"),
        metric_value,
    ));
    reg.register(metric_field(
        "disk_usage",
        "system.filesystem.utilization",
        Some("%"),
        metric_value,
    ));
    reg.register(metric_field(
        "disk_bytes",
        "system.filesystem.usage",
        Some("B"),
        metric_value,
    ));
    reg.register(metric_field(
        "disk_io",
        "system.disk.io",
        Some("B/s"),
        metric_value,
    ));
    reg.register(metric_field(
        "disk_iops",
        "system.disk.operations",
        Some("ops/s"),
        metric_value,
    ));
    reg.register(metric_field(
        "network_io",
        "system.network.io",
        Some("B/s"),
        metric_value,
    ));
    reg.register(metric_field(
        "network_packets",
        "system.network.packets",
        Some("packets/s"),
        metric_value,
    ));
    reg.register(metric_field(
        "network_latency",
        "system.network.latency",
        Some("ms"),
        metric_value,
    ));
    reg.register(metric_field(
        "network_jitter",
        "system.network.jitter",
        Some("ms"),
        metric_value,
    ));
    reg.register(metric_field(
        "packet_loss",
        "system.network.packet_loss",
        Some("%"),
        metric_value,
    ));
    reg.register(metric_field(
        "temperature",
        "system.temperature",
        Some("°C"),
        metric_value,
    ));
    reg.register(metric_field(
        "battery_charge",
        "system.battery.charge",
        Some("%"),
        metric_value,
    ));
    reg.register(metric_field(
        "battery_voltage",
        "system.battery.voltage",
        Some("V"),
        metric_value,
    ));
    reg.register(metric_field(
        "battery_current",
        "system.battery.current",
        Some("A"),
        metric_value,
    ));
    reg.register(metric_field(
        "battery_temperature",
        "system.battery.temperature",
        Some("°C"),
        metric_value,
    ));

    // ── Process metrics ─────────────────────────────────────────────
    reg.register(metric_field(
        "process_cpu",
        "process.cpu.utilization",
        Some("%"),
        metric_value,
    ));
    reg.register(metric_field(
        "process_memory",
        "process.memory.usage",
        Some("B"),
        metric_value,
    ));

    // ── otel_logs fields ────────────────────────────────────────────
    let body = profile.col("body", "Body");
    let severity_text = profile.col("severity_text", "SeverityText");
    let severity_number = profile.col("severity_number", "SeverityNumber");

    reg.register(simple("message", "otel_logs", body));
    reg.register(simple("severity", "otel_logs", severity_text));
    reg.register(simple("severity_number", "otel_logs", severity_number));
    reg.register(FieldDef {
        name: "log_service".into(),
        source_table: "otel_logs".into(),
        column: service_name.into(),
        ..Default::default()
    });

    // ── topic_messages fields ───────────────────────────────────────
    reg.register(simple("topic_name", "topic_messages", "topic_name"));
    reg.register(simple("robot_id", "topic_messages", "robot_id"));
    reg.register(simple("message_type", "topic_messages", "message_type"));

    // ── tf_states fields ────────────────────────────────────────────
    // Stores TF2 transform broadcasts (parent_frame → child_frame) for spatial
    // queries like `FROM tf WHERE parent_frame = 'base_link' AND child_frame = 'tool0'`.
    // All columns are bare typed columns; robot_id is direct (see `has_direct_robot_id`).
    reg.register(simple("timestamp", "tf_states", "timestamp"));
    reg.register(simple("org_id", "tf_states", "org_id"));
    reg.register(simple("robot_id", "tf_states", "robot_id"));
    reg.register(simple("parent_frame", "tf_states", "parent_frame"));
    reg.register(simple("child_frame", "tf_states", "child_frame"));
    reg.register(simple("translation_x", "tf_states", "translation_x"));
    reg.register(simple("translation_y", "tf_states", "translation_y"));
    reg.register(simple("translation_z", "tf_states", "translation_z"));
    reg.register(simple("rotation_x", "tf_states", "rotation_x"));
    reg.register(simple("rotation_y", "tf_states", "rotation_y"));
    reg.register(simple("rotation_z", "tf_states", "rotation_z"));
    reg.register(simple("rotation_w", "tf_states", "rotation_w"));

    // ── mcap_metadata fields ────────────────────────────────────────
    reg.register(simple("session_id", "mcap_metadata", "session_id"));
    reg.register(simple("file_uri", "mcap_metadata", "file_uri"));

    // ── node_graph_edges fields ─────────────────────────────────────
    // Stores ROS2 node-graph pub/sub edges (one node publishing to a topic that
    // another node subscribes to) for connectivity queries like
    // `FROM node_graph WHERE topic = '/scan' AND compatible = false`.
    // All columns are bare typed columns; robot_id is direct (see `has_direct_robot_id`).
    reg.register(simple("timestamp", "node_graph_edges", "timestamp"));
    reg.register(simple("org_id", "node_graph_edges", "org_id"));
    reg.register(simple("robot_id", "node_graph_edges", "robot_id"));
    reg.register(simple("source_node", "node_graph_edges", "source_node"));
    reg.register(simple("target_node", "node_graph_edges", "target_node"));
    reg.register(simple("topic", "node_graph_edges", "topic"));
    reg.register(simple("message_type", "node_graph_edges", "message_type"));
    reg.register(simple("publisher_qos", "node_graph_edges", "publisher_qos"));
    reg.register(simple(
        "subscriber_qos",
        "node_graph_edges",
        "subscriber_qos",
    ));
    reg.register(simple("rate_hz", "node_graph_edges", "rate_hz"));
    reg.register(simple("compatible", "node_graph_edges", "compatible"));

    // ── joint_states fields ─────────────────────────────────────────
    // Stores ROS2 `/joint_states` samples (per-joint position/velocity/effort)
    // for queries like `FROM joints WHERE joint_name = 'shoulder' AND effort > 10`.
    // All columns are bare typed columns; robot_id is direct (see `has_direct_robot_id`).
    reg.register(simple("timestamp", "joint_states", "timestamp"));
    reg.register(simple("org_id", "joint_states", "org_id"));
    reg.register(simple("robot_id", "joint_states", "robot_id"));
    reg.register(simple("joint_name", "joint_states", "joint_name"));
    reg.register(simple("position", "joint_states", "position"));
    reg.register(simple("velocity", "joint_states", "velocity"));
    reg.register(simple("effort", "joint_states", "effort"));

    reg
}

/// Convenience: build the default registry (OtelPostgres profile).
pub fn default_otel_registry() -> FieldRegistry {
    otel_registry(SchemaProfile::OtelPostgres)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn simple(name: &str, table: &str, column: &str) -> FieldDef {
    FieldDef {
        name: name.into(),
        source_table: table.into(),
        column: column.into(),
        ..Default::default()
    }
}

fn map_field(name: &str, table: &str, map_column: &str, map_key: &str) -> FieldDef {
    FieldDef {
        name: name.into(),
        source_table: table.into(),
        column: map_column.into(),
        is_map_access: true,
        map_column: Some(map_column.into()),
        map_key: Some(map_key.into()),
        ..Default::default()
    }
}

/// A generic concept alias: prefers `primary_key`, falling back (via `COALESCE`)
/// to each of `fallback_keys` in order. Used to decouple the query vocabulary
/// from ROS-only terms — e.g. `component` prefers `robot.component` but resolves
/// to `ros.node` for ROS data (ROB-432).
fn map_field_fallback(
    name: &str,
    table: &str,
    map_column: &str,
    primary_key: &str,
    fallback_keys: &[&str],
) -> FieldDef {
    FieldDef {
        name: name.into(),
        source_table: table.into(),
        column: map_column.into(),
        is_map_access: true,
        map_column: Some(map_column.into()),
        map_key: Some(primary_key.into()),
        fallback_keys: fallback_keys.iter().map(|k| (*k).to_string()).collect(),
        ..Default::default()
    }
}

fn metric_field(
    name: &str,
    metric_name: &str,
    storage_unit: Option<&str>,
    value_column: &str,
) -> FieldDef {
    FieldDef {
        name: name.into(),
        source_table: "otel_metrics".into(),
        column: value_column.into(),
        storage_unit: storage_unit.map(|s| s.into()),
        metric_filter: Some(metric_name.into()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_profile_uses_lowercase() {
        let reg = otel_registry(SchemaProfile::OtelPostgres);
        let dur = reg.resolve("duration").unwrap();
        assert_eq!(dur.column, "duration");
        let status = reg.resolve("status").unwrap();
        assert_eq!(status.column, "status_code");
    }

    #[test]
    fn clickhouse_profile_uses_pascal_case() {
        let reg = otel_registry(SchemaProfile::OtelClickhouse);
        let dur = reg.resolve("duration").unwrap();
        assert_eq!(dur.column, "Duration");
        let status = reg.resolve("status").unwrap();
        assert_eq!(status.column, "StatusCode");
    }

    #[test]
    fn default_registry_is_postgres() {
        let reg = default_otel_registry();
        let dur = reg.resolve("duration").unwrap();
        assert_eq!(dur.column, "duration");
    }

    #[test]
    fn registry_has_table_names() {
        let reg = default_otel_registry();
        assert_eq!(
            reg.table_name(&crate::ast::DataSource::Traces),
            Some("otel_traces")
        );
    }

    #[test]
    fn map_field_access() {
        let reg = default_otel_registry();
        let node = reg.resolve("node").unwrap();
        assert!(node.is_map_access);
        assert_eq!(node.map_key.as_deref(), Some("ros.node"));
    }

    // ── ROB-432: robot.* vocabulary + generic aliases ───────────────────

    #[test]
    fn robot_concept_keys_registered() {
        let reg = default_otel_registry();
        for key in [
            "robot.action.result",
            "robot.action.goal_id",
            "robot.component",
            "robot.transform.parent",
            "robot.joint.name",
            "robot.trajectory.point_count",
            "robot.target.frame",
            "robot.object.id",
        ] {
            let f = reg
                .resolve(key)
                .unwrap_or_else(|| panic!("missing concept key {key}"));
            assert!(f.is_map_access, "{key} should be span-attr map access");
            assert_eq!(f.map_key.as_deref(), Some(key));
            assert_eq!(f.source_table, "otel_traces");
        }
    }

    #[test]
    fn generic_aliases_prefer_robot_then_ros() {
        let reg = default_otel_registry();
        let component = reg.resolve("component").unwrap();
        assert_eq!(component.map_key.as_deref(), Some("robot.component"));
        assert_eq!(component.fallback_keys, vec!["ros.node".to_string()]);

        let action = reg.resolve("action").unwrap();
        assert_eq!(action.map_key.as_deref(), Some("robot.action.name"));
        assert_eq!(action.fallback_keys, vec!["ros.action.name".to_string()]);

        // `channel` has no portable robot.* key yet — single ros.topic mapping.
        let channel = reg.resolve("channel").unwrap();
        assert_eq!(channel.map_key.as_deref(), Some("ros.topic"));
        assert!(channel.fallback_keys.is_empty());
    }

    #[test]
    fn ros_field_names_unchanged() {
        // Back-compat: the classic ROS field names still resolve as before.
        let reg = default_otel_registry();
        assert_eq!(
            reg.resolve("node").unwrap().map_key.as_deref(),
            Some("ros.node")
        );
        assert_eq!(
            reg.resolve("topic").unwrap().map_key.as_deref(),
            Some("ros.topic")
        );
        assert!(reg.resolve("node").unwrap().fallback_keys.is_empty());
    }
}
