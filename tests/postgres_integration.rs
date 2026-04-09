//! PostgreSQL integration tests — run against live database with fixture data.
//!
//! These tests are #[ignore] by default and require:
//! - DATABASE_URL env var pointing to a PostgreSQL database with fixtures loaded
//! - Run via: `just test-examples` (handles Docker + fixtures automatically)
//! - Or manually: `DATABASE_URL=postgresql://rosql:rosql@localhost:5432/rosql_examples cargo test --ignored --features postgres`

#![cfg(feature = "postgres")]

use rosql::drivers::sql::SqlBackend;
use rosql::drivers::{BackendCapabilities, ExecOptions, ROSQLBackend};

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://rosql:rosql@localhost:5432/rosql_examples".into())
}

async fn setup() -> SqlBackend {
    SqlBackend::new(&db_url())
        .await
        .expect("failed to connect to PostgreSQL")
}

async fn execute_query(query: &str) -> rosql::ROSQLResult {
    let backend = setup().await;
    let ast = rosql::parse(query).expect("parse failed");
    let opts = ExecOptions::default();
    backend.execute(&ast, &opts).await.expect("query failed")
}

// ---------------------------------------------------------------------------
// Query tests — all require fixtures to be loaded
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn query_error_traces() {
    let result = execute_query("FROM traces WHERE status = 'ERROR'").await;
    assert!(
        result.metadata.row_count > 0,
        "expected error traces from fixture data"
    );
    assert!(
        result.columns.iter().any(|c| c.name == "status_code"),
        "expected status_code column"
    );
}

#[tokio::test]
#[ignore]
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
#[ignore]
async fn query_health() {
    let result = execute_query("HEALTH()").await;
    // HEALTH() returns UNION ALL of 3 signal types
    assert_eq!(
        result.metadata.row_count, 3,
        "expected 3 signal types (traces, logs, metrics)"
    );
}

#[tokio::test]
#[ignore]
async fn query_trace() {
    let result = execute_query("TRACE 'trace-002'").await;
    assert_eq!(
        result.metadata.row_count, 4,
        "expected 4 spans for trace-002"
    );
}

#[tokio::test]
#[ignore]
async fn query_show_recording() {
    let result = execute_query("SHOW RECORDING").await;
    assert_eq!(
        result.metadata.row_count, 1,
        "expected 1 recording from fixtures"
    );
}

#[tokio::test]
#[ignore]
async fn query_path_deviation() {
    let result = execute_query("PATH DEVIATION FOR ROBOT 'robot_sim_001'").await;
    assert!(
        result.metadata.row_count > 0,
        "expected /odom trajectory points"
    );
}

#[tokio::test]
#[ignore]
async fn query_topic_alias() {
    let result = execute_query("FROM odom LIMIT 5").await;
    assert!(
        result.metadata.row_count > 0,
        "expected odom data from topic_messages"
    );
    assert!(result.metadata.row_count <= 5, "LIMIT 5 should cap results");
}

#[tokio::test]
#[ignore]
async fn query_aggregation() {
    let result = execute_query("SELECT COUNT(*) FROM traces").await;
    assert_eq!(result.metadata.row_count, 1, "COUNT should return 1 row");
    // The count value should be 11 (total traces in fixtures)
    if let Some(row) = result.rows.first() {
        if let Some(serde_json::Value::Number(n)) = row.first() {
            assert_eq!(n.as_i64().unwrap(), 11, "expected 11 total traces");
        }
    }
}

// ---------------------------------------------------------------------------
// Capability error tests — these don't need fixtures
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn capability_error_no_topics() {
    let backend = SqlBackend::new_with_capabilities(
        &db_url(),
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
#[ignore]
async fn capability_error_no_recordings() {
    let backend = SqlBackend::new_with_capabilities(
        &db_url(),
        BackendCapabilities {
            topic_data: true,
            recording_index: false,
        },
    )
    .await
    .expect("failed to connect");

    let ast = rosql::parse("SHOW RECORDING").unwrap();
    let opts = ExecOptions::default();
    let err = backend.execute(&ast, &opts).await.unwrap_err();
    assert!(
        matches!(err, rosql::ROSQLError::DataSourceUnavailable { .. }),
        "expected DataSourceUnavailable, got: {err}"
    );
}
