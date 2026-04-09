//! DuckDB integration tests — run against an in-memory database with fixture data.
//!
//! No Docker required. DuckDB is embedded, so these tests run in CI without
//! any external services.
//!
//! Run with: `cargo test --features duckdb`

#![cfg(feature = "duckdb")]

use rosql::drivers::sql::SqlBackend;
use rosql::drivers::{BackendCapabilities, ExecOptions, ROSQLBackend};

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/duckdb/fixtures");

/// Load fixture SQL files into a temp database, return the duckdb:// URL.
///
/// SqlBackend::new() opens its own connection, so we can't share an in-memory
/// DB. Instead we write to a file inside a temp directory, drop the raw
/// connection, then let SqlBackend open it.
fn setup_fixture_db() -> (tempfile::TempDir, String) {
    let tmpdir = tempfile::tempdir().expect("failed to create temp dir");
    let path = tmpdir.path().join("rosql_test.db");

    // DuckDB creates the file — the path must not already exist
    let conn = duckdb::Connection::open(&path).expect("failed to open duckdb");
    let path = path.to_str().expect("non-UTF8 temp path").to_string();
    for n in 1..=6 {
        let fixture = format!("{FIXTURE_DIR}/{n:02}_*.sql");
        let matches: Vec<_> = glob::glob(&fixture)
            .expect("glob failed")
            .filter_map(|p| p.ok())
            .collect();
        for fixture_path in matches {
            let sql = std::fs::read_to_string(&fixture_path)
                .unwrap_or_else(|e| panic!("failed to read {fixture_path:?}: {e}"));
            conn.execute_batch(&sql)
                .unwrap_or_else(|e| panic!("failed to execute {fixture_path:?}: {e}"));
        }
    }

    let url = format!("duckdb://{path}");
    (tmpdir, url)
}

async fn setup(url: &str) -> SqlBackend {
    SqlBackend::new(url)
        .await
        .expect("failed to connect to DuckDB")
}

async fn execute_query(url: &str, query: &str) -> rosql::ROSQLResult {
    let backend = setup(url).await;
    let ast = rosql::parse(query).expect("parse failed");
    let opts = ExecOptions::default();
    backend.execute(&ast, &opts).await.expect("query failed")
}


// ---------------------------------------------------------------------------
// Query tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn query_error_traces() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "FROM traces WHERE status = 'ERROR'").await;
    assert!(
        result.metadata.row_count > 0,
        "expected error traces from fixture data"
    );
    assert!(
        result.columns.iter().any(|c| c.name == "status_code"),
        "expected status_code column, got: {:?}",
        result.columns.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn query_trace_recursive_cte() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "TRACE 'trace-002'").await;
    assert!(
        result.metadata.row_count > 0,
        "expected spans in trace tree"
    );
    // trace-002 has root → bt_navigator → controller → costmap = 4 spans
    assert_eq!(
        result.metadata.row_count, 4,
        "expected 4 spans in trace tree"
    );
}


#[tokio::test]
async fn query_trace() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "TRACE 'trace-002'").await;
    assert_eq!(
        result.metadata.row_count, 4,
        "expected 4 spans for trace-002"
    );
}

#[tokio::test]
async fn query_show_recording() {
    let (_tmp, url) = setup_fixture_db();
    // SHOW RECORDING is deprecated — use FROM recordings instead
    let result = execute_query(&url, "FROM recordings LIMIT 5").await;
    assert_eq!(
        result.metadata.row_count, 1,
        "expected 1 recording from fixtures"
    );
}

#[tokio::test]
async fn query_path_deviation() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "PATH DEVIATION FOR ROBOT 'robot_sim_001'").await;
    assert!(
        result.metadata.row_count > 0,
        "expected /odom trajectory points"
    );
}

#[tokio::test]
async fn query_topic_alias() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "FROM odom LIMIT 5").await;
    assert!(
        result.metadata.row_count > 0,
        "expected odom data from topic_messages"
    );
    assert!(result.metadata.row_count <= 5, "LIMIT 5 should cap results");
}

#[tokio::test]
async fn query_aggregation() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "SELECT COUNT(*) FROM traces").await;
    assert_eq!(result.metadata.row_count, 1, "COUNT should return 1 row");
    if let Some(row) = result.rows.first() {
        if let Some(serde_json::Value::Number(n)) = row.first() {
            assert_eq!(n.as_i64().unwrap(), 11, "expected 11 total traces");
        }
    }
}

// ---------------------------------------------------------------------------
// Capability error tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn capability_error_no_topics() {
    let (_tmp, url) = setup_fixture_db();
    let backend = SqlBackend::new_with_capabilities(
        &url,
        BackendCapabilities {
            topic_data: false,
            recording_index: true,
        },
    )
    .await
    .expect("failed to connect");

    let ast = rosql::parse("PATH DEVIATION FOR ROBOT 'r1'").unwrap();
    let opts = ExecOptions::default();
    let err = backend.execute(&ast, &opts).await.unwrap_err();
    assert!(
        matches!(err, rosql::ROSQLError::DataSourceUnavailable { .. }),
        "expected DataSourceUnavailable, got: {err}"
    );
}

#[tokio::test]
async fn capability_error_no_recordings() {
    let (_tmp, url) = setup_fixture_db();
    let backend = SqlBackend::new_with_capabilities(
        &url,
        BackendCapabilities {
            topic_data: true,
            recording_index: false,
        },
    )
    .await
    .expect("failed to connect");

    let ast = rosql::parse("FROM recordings LIMIT 5").unwrap();
    let opts = ExecOptions::default();
    let err = backend.execute(&ast, &opts).await.unwrap_err();
    assert!(
        matches!(err, rosql::ROSQLError::DataSourceUnavailable { .. }),
        "expected DataSourceUnavailable, got: {err}"
    );
}

// ── SHOW TOPICS / SHOW NODES / SHOW NODE GRAPH integration tests ─────────────

#[tokio::test]
async fn query_show_topics() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "SHOW TOPICS SINCE 30 days ago").await;
    // Should return columns topic_name, message_type, avg_rate_hz, publishers, subscribers
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"topic_name"), "got: {col_names:?}");
    assert!(col_names.contains(&"publishers"), "got: {col_names:?}");
}

#[tokio::test]
async fn query_show_nodes() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "SHOW NODES SINCE 30 days ago").await;
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"node_name"), "got: {col_names:?}");
    assert!(col_names.contains(&"error_count"), "got: {col_names:?}");
}

#[tokio::test]
async fn query_show_node_graph() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "SHOW NODE GRAPH SINCE 30 days ago").await;
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"source_node"), "got: {col_names:?}");
    assert!(col_names.contains(&"target_node"), "got: {col_names:?}");
    assert!(col_names.contains(&"topic"), "got: {col_names:?}");
}

// ── TIMESERIES integration tests ─────────────────────────────────────────────

#[tokio::test]
async fn query_timeseries_basic() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(
        &url,
        "SELECT COUNT(*) FROM traces TIMESERIES 1 hour SINCE 30 days ago",
    )
    .await;
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"time_bucket"), "got: {col_names:?}");
    assert!(
        !result.rows.is_empty(),
        "TIMESERIES should return at least one row"
    );
}

#[tokio::test]
async fn query_timeseries_with_facet() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(
        &url,
        "SELECT COUNT(*) FROM traces TIMESERIES 1 hour FACET action_name SINCE 30 days ago",
    )
    .await;
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"time_bucket"), "got: {col_names:?}");
}

// ── Health dashboard composable query integration tests ──────────────────────
// These five tests cover the shapes that replaced HEALTH() per issue #61.

/// Shape 1: Node liveness — already covered by query_show_nodes above.

/// Shape 2: Error rate with FACET
#[tokio::test]
async fn query_error_rate_facet() {
    let (_tmp, url) = setup_fixture_db();
    // FACET service_name: service_name is a real column on otel_traces.
    let result = execute_query(
        &url,
        "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' FACET service_name SINCE 30 days ago",
    )
    .await;
    assert!(
        result.metadata.row_count > 0,
        "expected error counts per service"
    );
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"service_name"), "got: {col_names:?}");
    // DuckDB names COUNT(*) as "count_star()" internally; we just verify there are 2 columns.
    assert_eq!(col_names.len(), 2, "expected service_name + count column, got: {col_names:?}");
}

/// Shape 3: Action success/failure via ACTION_SUCCESS_RATE
#[tokio::test]
async fn query_action_success_rate() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(
        &url,
        "SELECT ACTION_SUCCESS_RATE('/navigate_to_pose') FROM traces SINCE 30 days ago",
    )
    .await;
    // ACTION_SUCCESS_RATE with arg compiles as a scalar subquery — returns one row per outer row.
    // Verify it executes without error and returns a non-null numeric value.
    assert!(
        result.metadata.row_count > 0,
        "ACTION_SUCCESS_RATE should return rows"
    );
    if let Some(row) = result.rows.first() {
        if let Some(v) = row.first() {
            assert!(!v.is_null(), "expected a numeric success rate, got null");
        }
    }
}

/// Shape 4: Resource utilization
#[tokio::test]
async fn query_resource_utilization() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(
        &url,
        "SELECT AVG(metric_value) FROM metrics WHERE metric_name = 'system.cpu.total_usage_pct' SINCE 30 days ago",
    )
    .await;
    assert_eq!(result.metadata.row_count, 1, "AVG should return 1 row");
    if let Some(row) = result.rows.first() {
        if let Some(serde_json::Value::Number(n)) = row.first() {
            let avg = n.as_f64().unwrap();
            assert!(avg > 0.0, "expected non-zero average CPU usage, got {avg}");
        }
    }
}

/// Shape 5: Log severity distribution
#[tokio::test]
async fn query_log_severity_facet() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(
        &url,
        "SELECT COUNT(*) FROM logs FACET severity SINCE 30 days ago",
    )
    .await;
    assert!(
        result.metadata.row_count > 0,
        "expected severity counts from fixture logs"
    );
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    // `severity` maps to the `severity_text` column in otel_logs.
    assert!(
        col_names.contains(&"severity_text"),
        "expected severity_text column, got: {col_names:?}"
    );
    assert_eq!(col_names.len(), 2, "expected severity_text + count column, got: {col_names:?}");
}

/// TOPIC_RATE — publishes rate from otel_metrics
#[tokio::test]
async fn query_topic_rate() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(
        &url,
        "SELECT TOPIC_RATE('/cmd_vel') FROM metrics SINCE 30 days ago",
    )
    .await;
    // TOPIC_RATE compiles as a scalar subquery — returns one row per outer metric row.
    // Verify it executes without error and returns a non-null numeric value.
    assert!(
        result.metadata.row_count > 0,
        "TOPIC_RATE should return rows"
    );
    if let Some(row) = result.rows.first() {
        if let Some(v) = row.first() {
            assert!(
                !v.is_null(),
                "expected a non-null topic rate for /cmd_vel, got null"
            );
        }
    }
}
