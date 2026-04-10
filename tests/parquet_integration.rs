//! Parquet backend integration tests — run against pre-built Parquet fixture files.
//!
//! No Docker required. DuckDB is embedded and reads local Parquet files directly.
//!
//! # Fixture layout
//!
//! ```text
//! examples/parquet/fixtures/
//!   traces/otel_traces.parquet
//!   logs/otel_logs.parquet
//!   metrics/otel_metrics.parquet
//!   topic_messages/topic_messages.parquet
//!   mcap_metadata/mcap_metadata.parquet
//! ```
//!
//! Fixtures are generated from the SQL fixtures in `examples/duckdb/fixtures/` by
//! running `examples/parquet/generate_fixtures.sh`. Commit the `.parquet` files
//! alongside the SQL sources.
//!
//! Run with: `cargo test --features duckdb`

#![cfg(feature = "duckdb")]

use rosql::drivers::sql::SqlBackend;
use rosql::drivers::{BackendCapabilities, ExecOptions, ROSQLBackend};

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/parquet/fixtures");

async fn setup() -> SqlBackend {
    SqlBackend::from_parquet(FIXTURE_DIR)
        .await
        .expect("failed to create Parquet backend from fixtures")
}

async fn execute_query(query: &str) -> rosql::ROSQLResult {
    let backend = setup().await;
    let ast = rosql::parse(query).expect("parse failed");
    let opts = ExecOptions::default();
    backend.execute(&ast, &opts).await.expect("query failed")
}

// ---------------------------------------------------------------------------
// Query tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn query_error_traces() {
    let result = execute_query("FROM traces WHERE status = 'ERROR'").await;
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
    let result = execute_query("TRACE 'trace-002'").await;
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
    let result = execute_query("TRACE 'trace-002'").await;
    assert_eq!(
        result.metadata.row_count, 4,
        "expected 4 spans for trace-002"
    );
}

#[tokio::test]
async fn query_show_recording() {
    let result = execute_query("FROM recordings LIMIT 5").await;
    assert_eq!(
        result.metadata.row_count, 1,
        "expected 1 recording from fixtures"
    );
}

#[tokio::test]
async fn query_path_deviation() {
    let result = execute_query("PATH DEVIATION FOR ROBOT 'robot_sim_001'").await;
    assert!(
        result.metadata.row_count > 0,
        "expected /odom trajectory points"
    );
}

#[tokio::test]
async fn query_topic_alias() {
    let result = execute_query("FROM odom LIMIT 5").await;
    assert!(
        result.metadata.row_count > 0,
        "expected odom data from topic_messages"
    );
    assert!(result.metadata.row_count <= 5, "LIMIT 5 should cap results");
}

#[tokio::test]
async fn query_aggregation() {
    let result = execute_query("SELECT COUNT(*) FROM traces").await;
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
    let backend = SqlBackend::from_parquet_with_capabilities(
        FIXTURE_DIR,
        BackendCapabilities {
            topic_data: false,
            recording_index: true,
        },
    )
    .await
    .expect("failed to create parquet backend");

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
    let backend = SqlBackend::from_parquet_with_capabilities(
        FIXTURE_DIR,
        BackendCapabilities {
            topic_data: true,
            recording_index: false,
        },
    )
    .await
    .expect("failed to create parquet backend");

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
    let result = execute_query("SHOW TOPICS SINCE 30 days ago").await;
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"topic_name"), "got: {col_names:?}");
    assert!(col_names.contains(&"publishers"), "got: {col_names:?}");
}

#[tokio::test]
async fn query_show_nodes() {
    let result = execute_query("SHOW NODES SINCE 30 days ago").await;
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"node_name"), "got: {col_names:?}");
    assert!(col_names.contains(&"error_count"), "got: {col_names:?}");
}

#[tokio::test]
async fn query_show_node_graph() {
    let result = execute_query("SHOW NODE GRAPH SINCE 30 days ago").await;
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"source_node"), "got: {col_names:?}");
    assert!(col_names.contains(&"target_node"), "got: {col_names:?}");
    assert!(col_names.contains(&"topic"), "got: {col_names:?}");
}

// ── TIMESERIES integration tests ─────────────────────────────────────────────

#[tokio::test]
async fn query_timeseries_basic() {
    let result = execute_query(
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
    let result = execute_query(
        "SELECT COUNT(*) FROM traces TIMESERIES 1 hour FACET action_name SINCE 30 days ago",
    )
    .await;
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"time_bucket"), "got: {col_names:?}");
}

// ── Health dashboard composable query integration tests ──────────────────────

/// Error rate with FACET
#[tokio::test]
async fn query_error_rate_facet() {
    let result = execute_query(
        "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' FACET service_name SINCE 30 days ago",
    )
    .await;
    assert!(
        result.metadata.row_count > 0,
        "expected error counts per service"
    );
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(col_names.contains(&"service_name"), "got: {col_names:?}");
    assert_eq!(
        col_names.len(),
        2,
        "expected service_name + count column, got: {col_names:?}"
    );
}

/// Action success/failure via ACTION_SUCCESS_RATE
#[tokio::test]
async fn query_action_success_rate() {
    let result = execute_query(
        "SELECT ACTION_SUCCESS_RATE('/navigate_to_pose') FROM traces SINCE 30 days ago",
    )
    .await;
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

/// Resource utilization
#[tokio::test]
async fn query_resource_utilization() {
    let result = execute_query(
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

/// Log severity distribution
#[tokio::test]
async fn query_log_severity_facet() {
    let result = execute_query("SELECT COUNT(*) FROM logs FACET severity SINCE 30 days ago").await;
    assert!(
        result.metadata.row_count > 0,
        "expected severity counts from fixture logs"
    );
    let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert!(
        col_names.contains(&"severity_text"),
        "expected severity_text column, got: {col_names:?}"
    );
    assert_eq!(
        col_names.len(),
        2,
        "expected severity_text + count column, got: {col_names:?}"
    );
}

/// TOPIC_RATE — publish rate from otel_metrics
#[tokio::test]
async fn query_topic_rate() {
    let result = execute_query("SELECT TOPIC_RATE('/cmd_vel') FROM metrics SINCE 30 days ago").await;
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

// ---------------------------------------------------------------------------
// Parquet-specific tests
// ---------------------------------------------------------------------------

/// Verify that pointing at a directory with only some subdirectories populates
/// only the available views and marks missing capabilities accordingly.
#[tokio::test]
async fn parquet_missing_subdirectory_degrades_gracefully() {
    // Use the traces-only subdirectory by pointing at a path where only traces/ exists.
    // We synthesise this by creating a temp dir with only the traces subdirectory.
    let tmpdir = tempfile::tempdir().expect("failed to create temp dir");
    let traces_dir = tmpdir.path().join("traces");
    std::fs::create_dir_all(&traces_dir).unwrap();
    std::fs::copy(
        format!("{FIXTURE_DIR}/traces/otel_traces.parquet"),
        traces_dir.join("otel_traces.parquet"),
    )
    .unwrap();

    let url = tmpdir.path().to_str().unwrap();
    let backend = SqlBackend::from_parquet(url).await.expect("should succeed with partial fixtures");

    // topic_messages and mcap_metadata subdirectories are absent → capabilities false
    assert!(
        !backend.capabilities().topic_data,
        "topic_data should be false when topic_messages/ is absent"
    );
    assert!(
        !backend.capabilities().recording_index,
        "recording_index should be false when mcap_metadata/ is absent"
    );

    // Querying traces should still work
    let ast = rosql::parse("FROM traces WHERE status = 'ERROR'").unwrap();
    let opts = ExecOptions::default();
    let result = backend.execute(&ast, &opts).await.expect("trace query should succeed");
    assert!(result.metadata.row_count > 0, "expected error traces");
}

/// Pointing at a nonexistent directory should return a clear error.
#[tokio::test]
async fn parquet_nonexistent_directory_gives_error() {
    // DuckDB read_parquet() with a glob that matches no files returns an error at view creation
    // time (or at query time if the view was silently skipped). The backend should still
    // construct successfully — the error surfaces when a query is executed.
    let result = SqlBackend::from_parquet("/tmp/rosql-nonexistent-1234567890").await;
    // Either construction fails, or the backend is created but all capabilities are absent.
    match result {
        Err(_) => {
            // Acceptable: explicit error at construction time
        }
        Ok(backend) => {
            assert!(
                !backend.capabilities().topic_data && !backend.capabilities().recording_index,
                "expected no capabilities for an empty/nonexistent directory"
            );
        }
    }
}

/// Verify that an s3:// URL is accepted by from_parquet() without credentials
/// and that the resulting error mentions httpfs or S3 rather than an internal panic.
#[tokio::test]
async fn parquet_s3_url_without_credentials_gives_informative_error() {
    // This test does NOT require real S3 access. It verifies:
    // 1. The code path reaches httpfs setup (not a silent no-op or panic).
    // 2. The error (if any) is a ROSQLError::DriverError, not an unhandled panic.
    // In CI without credentials, the views will fail to create or probe will find
    // no accessible tables — either outcome is acceptable.
    let result = SqlBackend::from_parquet("s3://rosql-test-does-not-exist/fixtures").await;
    match result {
        Err(e) => {
            // Should be a DriverError, not a panic or other unhandled condition
            assert!(
                matches!(e, rosql::ROSQLError::DriverError { .. }),
                "expected DriverError for inaccessible S3 URL, got: {e:?}"
            );
        }
        Ok(backend) => {
            // If httpfs loaded but credentials are absent, capabilities should be false
            assert!(
                !backend.capabilities().topic_data && !backend.capabilities().recording_index,
                "expected no capabilities without valid S3 credentials"
            );
        }
    }
}
