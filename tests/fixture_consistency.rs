//! Fixture consistency tests — validates the integrity of the DuckDB SQL fixture dataset.
//!
//! These tests load all 8 fixture files into an in-memory DuckDB database and verify:
//! - Cross-table referential integrity (trace IDs, span IDs, robot IDs)
//! - Temporal monotonicity for time-series data
//! - Presence of data required by each showcase query
//!
//! Run with: `cargo test --features duckdb`

#![cfg(feature = "duckdb")]

use duckdb::Connection;

const FIXTURE_FILES: &[&str] = &[
    "examples/duckdb/fixtures/01_schema.sql",
    "examples/duckdb/fixtures/02_traces.sql",
    "examples/duckdb/fixtures/03_logs.sql",
    "examples/duckdb/fixtures/04_metrics.sql",
    "examples/duckdb/fixtures/05_topic_messages.sql",
    "examples/duckdb/fixtures/06_mcap_metadata.sql",
    "examples/duckdb/fixtures/07_events.sql",
    "examples/duckdb/fixtures/08_baseline.sql",
    "examples/duckdb/fixtures/10_tf_states.sql",
];

fn load_fixtures() -> Connection {
    let conn = Connection::open_in_memory().expect("failed to open in-memory DuckDB");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in FIXTURE_FILES {
        let path = format!("{manifest_dir}/{file}");
        let sql = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
        conn.execute_batch(&sql)
            .unwrap_or_else(|e| panic!("failed to execute fixture {path}: {e}"));
    }
    conn
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0))
        .unwrap_or_else(|e| panic!("query failed: {sql}\n{e}"))
}

// ── Referential integrity ────────────────────────────────────────────────────

#[test]
fn all_log_trace_ids_exist_in_traces() {
    let conn = load_fixtures();
    let orphans: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM otel_logs l \
         WHERE l.trace_id NOT IN (SELECT DISTINCT trace_id FROM otel_traces)",
    );
    assert_eq!(
        orphans, 0,
        "found {orphans} log entries referencing unknown trace_ids"
    );
}

#[test]
fn all_span_parent_ids_form_valid_trees() {
    let conn = load_fixtures();
    // Every non-empty parent_span_id must refer to a span in the same trace.
    let dangling: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM otel_traces child \
         WHERE child.parent_span_id IS NOT NULL \
           AND child.parent_span_id != '' \
           AND NOT EXISTS ( \
             SELECT 1 FROM otel_traces parent \
             WHERE parent.span_id = child.parent_span_id \
               AND parent.trace_id = child.trace_id \
           )",
    );
    assert_eq!(
        dangling, 0,
        "found {dangling} spans with parent_span_id that doesn't match any sibling span"
    );
}

#[test]
fn all_metric_robot_ids_exist_in_traces() {
    let conn = load_fixtures();
    // Every robot_id appearing in metrics must also appear in traces.
    let orphans: i64 = count(
        &conn,
        "SELECT COUNT(DISTINCT resource_attributes->>'robot.id') \
         FROM otel_metrics \
         WHERE (resource_attributes->>'robot.id') NOT IN ( \
           SELECT DISTINCT resource_attributes->>'robot.id' FROM otel_traces \
         )",
    );
    assert_eq!(
        orphans, 0,
        "found {orphans} robot_ids in metrics with no matching traces"
    );
}

#[test]
fn deployment_versions_match_trace_resource_attributes() {
    let conn = load_fixtures();
    // Each firmware version in ros2_events should appear in at least one trace's
    // resource_attributes service.version field.
    let unmatched: i64 = count(
        &conn,
        "SELECT COUNT(DISTINCT e.version) \
         FROM ros2_events e \
         WHERE e.event_type = 'firmware_deploy' \
           AND e.version != '' \
           AND e.version NOT IN ( \
             SELECT DISTINCT resource_attributes->>'service.version' FROM otel_traces \
           )",
    );
    assert_eq!(
        unmatched, 0,
        "found {unmatched} firmware versions in events not reflected in trace resource attributes"
    );
}

// ── Temporal consistency ─────────────────────────────────────────────────────

#[test]
fn battery_readings_are_temporally_ordered() {
    let conn = load_fixtures();
    // Battery readings for robot-amr-02 should have monotonically increasing timestamps.
    // We check that no reading has a timestamp earlier than the previous one.
    let disordered: i64 = count(
        &conn,
        "WITH ordered AS ( \
           SELECT timestamp, \
                  LAG(timestamp) OVER (ORDER BY timestamp) AS prev_ts \
           FROM topic_messages \
           WHERE robot_id = 'robot-amr-02' AND topic_name = '/battery_state' \
         ) \
         SELECT COUNT(*) FROM ordered WHERE timestamp < prev_ts",
    );
    assert_eq!(
        disordered, 0,
        "found {disordered} out-of-order battery readings"
    );
}

// ── Showcase query data requirements ────────────────────────────────────────

#[test]
fn trace_amr02_m3_exists_and_has_error_span() {
    let conn = load_fixtures();

    let trace_count: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM otel_traces WHERE trace_id = 'trace-amr02-m3'",
    );
    assert!(
        trace_count > 0,
        "trace-amr02-m3 must exist for showcase query 1"
    );

    let error_spans: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM otel_traces \
         WHERE trace_id = 'trace-amr02-m3' AND status_code = 'ERROR'",
    );
    assert!(
        error_spans > 0,
        "trace-amr02-m3 must have at least one ERROR span"
    );
}

#[test]
fn trace_amr02_m3_has_correlated_error_logs() {
    let conn = load_fixtures();
    let logs: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM otel_logs \
         WHERE trace_id = 'trace-amr02-m3' AND severity_text IN ('ERROR', 'WARN')",
    );
    assert!(
        logs > 0,
        "trace-amr02-m3 must have correlated ERROR/WARN logs for showcase query 2"
    );
}

#[test]
fn cpu_metrics_cover_all_three_robots() {
    let conn = load_fixtures();
    let robot_count: i64 = count(
        &conn,
        "SELECT COUNT(DISTINCT resource_attributes->>'robot.id') \
         FROM otel_metrics WHERE metric_name = 'system.cpu.utilization'",
    );
    assert_eq!(
        robot_count, 3,
        "cpu metrics must cover all 3 robots for showcase query 3 (got {robot_count})"
    );
}

#[test]
fn scan_topic_spans_exist_for_message_flow() {
    let conn = load_fixtures();
    let scan_spans: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM otel_traces \
         WHERE span_attributes->>'ros.topic' = '/scan' \
           AND CAST(resource_attributes AS VARCHAR) LIKE '%robot-amr-02%'",
    );
    assert!(
        scan_spans > 0,
        "must have /scan topic spans on robot-amr-02 for showcase query 4 (message flow)"
    );
}

#[test]
fn node_graph_spans_exist_for_robot_amr02() {
    let conn = load_fixtures();
    let topology_spans: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM otel_traces \
         WHERE CAST(span_attributes AS VARCHAR) LIKE '%ros.publisher_node%' \
           AND CAST(span_attributes AS VARCHAR) LIKE '%ros.subscriber_node%' \
           AND CAST(resource_attributes AS VARCHAR) LIKE '%robot-amr-02%'",
    );
    assert!(
        topology_spans > 0,
        "must have topology spans with publisher/subscriber nodes for showcase query 9 (node graph)"
    );
}

#[test]
fn battery_readings_below_threshold_exist() {
    let conn = load_fixtures();
    let low_battery: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM topic_messages \
         WHERE robot_id = 'robot-amr-02' \
           AND topic_name = '/battery_state' \
           AND CAST(fields->>'voltage' AS DOUBLE) < 11.5",
    );
    assert!(
        low_battery > 0,
        "must have battery readings below 11.5V for showcase query 8"
    );
}

#[test]
fn historical_baseline_exists_for_all_robots() {
    let conn = load_fixtures();
    // Baseline data should exist for all 3 robots in the historical window.
    let robots_with_baseline: i64 = count(
        &conn,
        "SELECT COUNT(DISTINCT resource_attributes->>'robot.id') \
         FROM otel_traces \
         WHERE timestamp < NOW()::TIMESTAMP - INTERVAL '6 days'",
    );
    assert_eq!(
        robots_with_baseline, 3,
        "baseline data must exist for all 3 robots for showcase query 7 (anomaly detection)"
    );
}

#[test]
fn path_deviation_odometry_exists_for_trace() {
    let conn = load_fixtures();
    let odom_messages: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM topic_messages \
         WHERE trace_id = 'trace-amr02-m3' AND topic_name = '/odom'",
    );
    assert!(
        odom_messages > 0,
        "must have /odom messages for trace-amr02-m3 for showcase query 6 (path deviation)"
    );
}
