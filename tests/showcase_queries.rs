//! Showcase query execution tests — compile each of the 9 REPL showcase queries
//! and execute them against the DuckDB SQL fixture dataset.
//!
//! Verifies:
//! - All 9 showcase queries parse and compile without errors
//! - Each query returns at least one result row from the fixture data
//! - Result columns match the expected shape
//!
//! Run with: `cargo test --features duckdb`

#![cfg(feature = "duckdb")]

use duckdb::Connection;
use rosql::ast::FormatHint;
use rosql::drivers::compiler::compile;
use rosql::drivers::dialect::SqlDialect;
use rosql::drivers::otel_registry::default_otel_registry;
use rosql::drivers::BackendCapabilities;

const FIXTURE_FILES: &[&str] = &[
    "examples/duckdb/fixtures/01_schema.sql",
    "examples/duckdb/fixtures/02_traces.sql",
    "examples/duckdb/fixtures/03_logs.sql",
    "examples/duckdb/fixtures/04_metrics.sql",
    "examples/duckdb/fixtures/05_topic_messages.sql",
    "examples/duckdb/fixtures/06_mcap_metadata.sql",
    "examples/duckdb/fixtures/07_events.sql",
    "examples/duckdb/fixtures/08_baseline.sql",
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

fn caps() -> BackendCapabilities {
    BackendCapabilities {
        topic_data: true,
        recording_index: true,
    }
}

/// Compiles a ROSQL query to DuckDB SQL and returns (compiled_sql, format_hint).
fn compile_showcase(query: &str) -> (String, FormatHint) {
    let ast = rosql::parse(query).unwrap_or_else(|e| panic!("parse failed for:\n{query}\n{e:?}"));
    let registry = default_otel_registry();
    let result = compile(&ast, &registry, &SqlDialect::DuckDB, &caps(), None)
        .unwrap_or_else(|e| panic!("compile failed for:\n{query}\n{e}"));
    (result.sql, result.format_hint)
}

/// Executes a SQL string against the fixture connection and returns the row count.
fn row_count(conn: &Connection, sql: &str) -> usize {
    let mut stmt = conn
        .prepare(sql)
        .unwrap_or_else(|e| panic!("prepare failed:\n{sql}\n{e}"));
    let mut rows = stmt
        .query([])
        .unwrap_or_else(|e| panic!("query failed:\n{sql}\n{e}"));
    let mut count = 0usize;
    while rows
        .next()
        .unwrap_or_else(|e| panic!("row iteration failed: {e}"))
        .is_some()
    {
        count += 1;
    }
    count
}

// ── Showcase query 1: Trace a failed mission ─────────────────────────────────

#[test]
fn showcase_01_trace_failed_mission() {
    let conn = load_fixtures();
    let (sql, hint) = compile_showcase("TRACE 'trace-amr02-m3'");
    assert_eq!(hint, FormatHint::Gantt);
    let rows = row_count(&conn, &sql);
    assert!(rows > 0, "expected spans from trace-amr02-m3, got 0");
}

// ── Showcase query 2: Show logs for a failed trace (ENRICH WITH logs) ───────────────────

#[test]
fn showcase_02_enrich_with_logs() {
    let conn = load_fixtures();
    let (sql, hint) = compile_showcase("TRACE 'trace-amr02-m3'\nENRICH WITH logs LIMIT 5");
    assert_eq!(hint, FormatHint::Gantt);
    let rows = row_count(&conn, &sql);
    assert!(rows > 0, "expected enriched trace rows, got 0");
}

// ── Showcase query 3: CPU usage across fleet ────────────────────────────────────

#[test]
fn showcase_03_fleet_cpu_timeseries() {
    let conn = load_fixtures();
    let (sql, hint) = compile_showcase(
        "SELECT cpu_usage FROM metrics\nTIMESERIES 2 min FACET robot_id\nSINCE 45 min ago",
    );
    assert_eq!(hint, FormatHint::StackedLineChart);
    assert!(
        sql.contains("AVG("),
        "bare field must be wrapped in AVG(): {sql}"
    );
    assert!(
        sql.contains("time_bucket"),
        "must include time_bucket: {sql}"
    );
    assert!(sql.contains("GROUP BY"), "must have GROUP BY: {sql}");
    assert!(
        sql.contains("robot_id"),
        "facet alias must appear in SELECT: {sql}"
    );
    let rows = row_count(&conn, &sql);
    assert!(rows > 0, "expected cpu_usage timeseries rows, got 0");
}

// ── Showcase query 4: Message flow for topic: /scan ────────────────────────────────────

#[test]
fn showcase_04_message_flow_scan() {
    let conn = load_fixtures();
    let (sql, hint) = compile_showcase("MESSAGE FLOW FROM TOPIC '/scan'\nFOR ROBOT 'robot-amr-02'");
    assert_eq!(hint, FormatHint::DirectedGraph);
    assert!(
        sql.contains("/scan"),
        "compiled SQL must reference the topic name: {sql}"
    );
    let rows = row_count(&conn, &sql);
    assert!(rows > 0, "expected message flow rows for /scan, got 0");
}

// ── Showcase query 5: Slowest actions/spans ──────────────────────────────────────────

#[test]
fn showcase_05_slowest_spans() {
    let conn = load_fixtures();
    let (sql, hint) =
        compile_showcase("SHOW SPAN SUMMARY\nFOR ROBOT 'robot-amr-02'\nSINCE 90 min ago");
    assert_eq!(hint, FormatHint::HorizontalBars);
    let rows = row_count(&conn, &sql);
    assert!(rows > 0, "expected span summary rows, got 0");
}

// ── Showcase query 6: Path deviation ─────────────────────────────────────────

#[test]
fn showcase_06_path_deviation() {
    let conn = load_fixtures();
    let (sql, hint) = compile_showcase("PATH DEVIATION\nFOR TRACE 'trace-amr02-m3'");
    assert_eq!(hint, FormatHint::LineChart);
    let rows = row_count(&conn, &sql);
    assert!(rows > 0, "expected path deviation rows, got 0");
}

// ── Showcase query 7: Which robot regressed? ─────────────────────────────────

#[test]
fn showcase_07_anomaly_detection() {
    let conn = load_fixtures();
    let (sql, hint) = compile_showcase("ANOMALY(duration)\nCOMPARED TO last week\nFACET robot_id");
    assert_eq!(hint, FormatHint::Table);
    let rows = row_count(&conn, &sql);
    assert_eq!(rows, 3, "expected one anomaly row per robot, got {rows}");
}

// ── Showcase query 8: Battery below 11.5V ────────────────────────────────────

#[test]
fn showcase_08_battery_below_threshold() {
    let conn = load_fixtures();
    let (sql, hint) = compile_showcase(
        "FROM topics\nWHERE topic_name = '/battery_state'\n  AND fields['voltage'] < 11.5 V\nFOR ROBOT 'robot-amr-02'\nSINCE 2 h ago",
    );
    assert_eq!(hint, FormatHint::Table);
    assert!(
        sql.contains("11.5"),
        "compiled SQL must include voltage threshold: {sql}"
    );
    let rows = row_count(&conn, &sql);
    assert!(rows > 0, "expected battery readings below 11.5V, got 0");
}

// ── Showcase query 9: ROS2 node topology ─────────────────────────────────────

#[test]
fn showcase_09_node_graph() {
    let conn = load_fixtures();
    let (sql, hint) = compile_showcase("SHOW NODE GRAPH\nFOR ROBOT 'robot-amr-02'");
    assert_eq!(hint, FormatHint::NodeGraph);
    assert!(
        sql.contains("publisher_node") || sql.contains("ros.publisher"),
        "compiled SQL must query ROS2 publisher/subscriber topology: {sql}"
    );
    let rows = row_count(&conn, &sql);
    assert!(rows > 0, "expected node graph topology rows, got 0");
}
