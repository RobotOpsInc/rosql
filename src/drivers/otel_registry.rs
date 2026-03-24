//! Default OTel field registry — maps ROSQL fields to standard OTel column names.

use super::field_registry::{FieldDef, FieldRegistry};

/// Build the default field registry for the standard OTel Collector schema.
pub fn default_otel_registry() -> FieldRegistry {
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

    // ── otel_traces fields ──────────────────────────────────────────
    reg.register(simple("trace_id", "otel_traces", "TraceId"));
    reg.register(simple("span_id", "otel_traces", "SpanId"));
    reg.register(simple("parent_span_id", "otel_traces", "ParentSpanId"));
    reg.register(simple("span_name", "otel_traces", "SpanName"));
    reg.register(simple("service", "otel_traces", "ServiceName"));
    reg.register(FieldDef {
        name: "duration".into(),
        source_table: "otel_traces".into(),
        column: "Duration".into(),
        storage_unit: Some("ns".into()),
        is_map_access: false,
        map_column: None,
        map_key: None,
        metric_filter: None,
    });
    reg.register(simple("status", "otel_traces", "StatusCode"));

    // ROS2 span attributes (map access)
    reg.register(map_field(
        "node",
        "otel_traces",
        "SpanAttributes",
        "ros.node",
    ));
    reg.register(map_field(
        "action_name",
        "otel_traces",
        "SpanAttributes",
        "ros.action.name",
    ));
    reg.register(map_field(
        "action_status",
        "otel_traces",
        "SpanAttributes",
        "ros.action.status",
    ));
    reg.register(map_field(
        "topic",
        "otel_traces",
        "SpanAttributes",
        "ros.topic",
    ));

    // ── otel_metrics fields ─────────────────────────────────────────
    reg.register(simple("metric_name", "otel_metrics", "MetricName"));
    reg.register(simple("metric_value", "otel_metrics", "Value"));

    reg.register(metric_field(
        "publish_rate",
        "ros2.topic.rx_rate_hz",
        Some("Hz"),
    ));
    reg.register(metric_field(
        "bandwidth",
        "ros2.topic.rx_bandwidth_bps",
        Some("B/s"),
    ));
    reg.register(metric_field(
        "cpu_usage",
        "system.cpu.total_usage_pct",
        None,
    ));
    reg.register(metric_field(
        "memory_usage",
        "system.memory.usage_pct",
        None,
    ));

    // ── otel_logs fields ────────────────────────────────────────────
    reg.register(simple("message", "otel_logs", "Body"));
    reg.register(simple("severity", "otel_logs", "SeverityText"));
    reg.register(simple("severity_number", "otel_logs", "SeverityNumber"));
    // "service" already registered for traces; logs also have ServiceName
    // We register a separate entry keyed by source table
    reg.register(FieldDef {
        name: "log_service".into(),
        source_table: "otel_logs".into(),
        column: "ServiceName".into(),
        storage_unit: None,
        is_map_access: false,
        map_column: None,
        map_key: None,
        metric_filter: None,
    });

    // ── topic_messages fields ───────────────────────────────────────
    reg.register(simple("topic_name", "topic_messages", "topic_name"));
    reg.register(simple("robot_id", "topic_messages", "robot_id"));
    reg.register(simple("message_type", "topic_messages", "message_type"));

    // ── mcap_metadata fields ────────────────────────────────────────
    reg.register(simple("session_id", "mcap_metadata", "session_id"));
    reg.register(simple("s3_key", "mcap_metadata", "s3_key"));

    reg
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn simple(name: &str, table: &str, column: &str) -> FieldDef {
    FieldDef {
        name: name.into(),
        source_table: table.into(),
        column: column.into(),
        storage_unit: None,
        is_map_access: false,
        map_column: None,
        map_key: None,
        metric_filter: None,
    }
}

fn map_field(name: &str, table: &str, map_column: &str, map_key: &str) -> FieldDef {
    FieldDef {
        name: name.into(),
        source_table: table.into(),
        column: map_column.into(),
        storage_unit: None,
        is_map_access: true,
        map_column: Some(map_column.into()),
        map_key: Some(map_key.into()),
        metric_filter: None,
    }
}

fn metric_field(name: &str, metric_name: &str, storage_unit: Option<&str>) -> FieldDef {
    FieldDef {
        name: name.into(),
        source_table: "otel_metrics".into(),
        column: "Value".into(),
        storage_unit: storage_unit.map(|s| s.into()),
        is_map_access: false,
        map_column: None,
        map_key: None,
        metric_filter: Some(metric_name.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_trace_fields() {
        let reg = default_otel_registry();
        let dur = reg.resolve("duration").unwrap();
        assert_eq!(dur.column, "Duration");
        assert_eq!(dur.storage_unit.as_deref(), Some("ns"));

        let node = reg.resolve("node").unwrap();
        assert!(node.is_map_access);
        assert_eq!(node.map_key.as_deref(), Some("ros.node"));
    }

    #[test]
    fn registry_has_metric_fields() {
        let reg = default_otel_registry();
        let pr = reg.resolve("publish_rate").unwrap();
        assert_eq!(pr.metric_filter.as_deref(), Some("ros2.topic.rx_rate_hz"));
    }

    #[test]
    fn registry_has_log_fields() {
        let reg = default_otel_registry();
        let msg = reg.resolve("message").unwrap();
        assert_eq!(msg.column, "Body");

        let sev = reg.resolve("severity").unwrap();
        assert_eq!(sev.column, "SeverityText");
    }

    #[test]
    fn registry_has_table_names() {
        let reg = default_otel_registry();
        assert_eq!(
            reg.table_name(&crate::ast::DataSource::Traces),
            Some("otel_traces")
        );
        assert_eq!(
            reg.table_name(&crate::ast::DataSource::Logs),
            Some("otel_logs")
        );
        assert_eq!(
            reg.table_name(&crate::ast::DataSource::Metrics),
            Some("otel_metrics")
        );
    }
}
