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
async fn query_message_journey() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "MESSAGE JOURNEY FOR TRACE 'trace-002'").await;
    assert!(
        result.metadata.row_count > 0,
        "expected spans in message journey"
    );
    // trace-002 has root → bt_navigator → controller → costmap = 4 spans
    assert_eq!(
        result.metadata.row_count, 4,
        "expected 4 spans in journey chain"
    );
}

#[tokio::test]
async fn query_health() {
    let (_tmp, url) = setup_fixture_db();
    let result = execute_query(&url, "HEALTH()").await;
    assert_eq!(
        result.metadata.row_count, 3,
        "expected 3 signal types (traces, logs, metrics)"
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
    let result = execute_query(&url, "SHOW RECORDING").await;
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

    let ast = rosql::parse("SHOW RECORDING").unwrap();
    let opts = ExecOptions::default();
    let err = backend.execute(&ast, &opts).await.unwrap_err();
    assert!(
        matches!(err, rosql::ROSQLError::DataSourceUnavailable { .. }),
        "expected DataSourceUnavailable, got: {err}"
    );
}
