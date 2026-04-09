//! Tests for issue #61: stub audit, safety defaults, and removed features.
//!
//! Verifies:
//!   - Implemented aggregation functions compile to correct SQL per dialect
//!   - Gated aggregation functions return NotImplemented
//!   - Gated compound clauses return NotImplemented
//!   - Default LIMIT 100 is applied and the metadata flag is set
//!   - Default LIMIT is NOT applied for exempt query types
//!   - OFFSET parses and compiles correctly
//!   - ALERT/DEFINE produce specific error messages

use rosql::drivers::compiler::compile;
use rosql::drivers::dialect::SqlDialect;
use rosql::drivers::otel_registry::default_otel_registry;
use rosql::drivers::BackendCapabilities;
use rosql::error::ROSQLError;

fn caps() -> BackendCapabilities {
    BackendCapabilities {
        topic_data: true,
        recording_index: true,
    }
}

fn compile_sql(query: &str, dialect: SqlDialect) -> String {
    let ast = rosql::parse(query).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let registry = default_otel_registry();
    compile(&ast, &registry, &dialect, &caps(), None)
        .unwrap_or_else(|e| panic!("compile failed: {e}"))
        .sql
}

fn compile_err(query: &str, dialect: SqlDialect) -> ROSQLError {
    let ast = rosql::parse(query).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let registry = default_otel_registry();
    compile(&ast, &registry, &dialect, &caps(), None).unwrap_err()
}

fn compile_with_default_limit(query: &str, default: u64) -> (String, bool) {
    let ast = rosql::parse(query).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let registry = default_otel_registry();
    let cr = compile(&ast, &registry, &SqlDialect::DuckDB, &caps(), Some(default))
        .unwrap_or_else(|e| panic!("compile failed: {e}"));
    (cr.sql, cr.default_limit_applied)
}

// ── Implemented aggregation functions ───────────────────────────────────────

#[test]
fn topic_rate_compiles_to_subquery() {
    let sql = compile_sql("SELECT TOPIC_RATE() FROM metrics", SqlDialect::PostgreSQL);
    assert!(sql.contains("otel_metrics"), "got: {sql}");
    assert!(sql.contains("ros2.topic.message_rate"), "got: {sql}");
    assert!(sql.contains("AVG"), "got: {sql}");
}

#[test]
fn topic_rate_with_topic_arg() {
    let sql = compile_sql(
        "SELECT TOPIC_RATE('/cmd_vel') FROM metrics",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("otel_metrics"), "got: {sql}");
    assert!(sql.contains("topic_name"), "got: {sql}");
    assert!(sql.contains("/cmd_vel"), "got: {sql}");
}

#[test]
fn action_success_rate_no_arg() {
    let sql = compile_sql(
        "SELECT ACTION_SUCCESS_RATE() FROM traces",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("succeeded"), "got: {sql}");
    assert!(sql.contains("CASE WHEN"), "got: {sql}");
    assert!(sql.contains("COUNT"), "got: {sql}");
    assert!(sql.contains("NULLIF"), "got: {sql}");
}

#[test]
fn action_success_rate_with_arg() {
    let sql = compile_sql(
        "SELECT ACTION_SUCCESS_RATE('/navigate_to_pose') FROM traces",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("navigate_to_pose"), "got: {sql}");
    assert!(sql.contains("succeeded"), "got: {sql}");
}

#[test]
fn moving_avg_window() {
    let sql = compile_sql(
        "SELECT MOVING_AVG(duration, 5) FROM traces",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("AVG"), "got: {sql}");
    assert!(sql.contains("OVER"), "got: {sql}");
    assert!(sql.contains("ROWS BETWEEN"), "got: {sql}");
    assert!(sql.contains("PRECEDING"), "got: {sql}");
    // Window of 5 → 4 PRECEDING
    assert!(sql.contains("4 PRECEDING"), "got: {sql}");
}

#[test]
fn derivative_compiles_lag() {
    let sql = compile_sql(
        "SELECT DERIVATIVE(metric_value) FROM metrics",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("LAG("), "got: {sql}");
    assert!(sql.contains("OVER"), "got: {sql}");
    assert!(sql.contains("NULLIF"), "got: {sql}");
}

#[test]
fn approx_count_distinct_pg() {
    let sql = compile_sql(
        "SELECT APPROX_COUNT_DISTINCT(span_name) FROM traces",
        SqlDialect::PostgreSQL,
    );
    // PG falls back to exact COUNT(DISTINCT ...)
    assert!(sql.contains("COUNT(DISTINCT"), "got: {sql}");
}

#[test]
fn approx_count_distinct_duckdb() {
    let sql = compile_sql(
        "SELECT APPROX_COUNT_DISTINCT(span_name) FROM traces",
        SqlDialect::DuckDB,
    );
    assert!(sql.contains("approx_count_distinct("), "got: {sql}");
}

#[test]
fn approx_percentile_pg() {
    let sql = compile_sql(
        "SELECT APPROX_PERCENTILE(duration, 95) FROM traces",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("PERCENTILE_CONT"), "got: {sql}");
    assert!(sql.contains("0.95"), "got: {sql}");
}

#[test]
fn approx_percentile_duckdb() {
    let sql = compile_sql(
        "SELECT APPROX_PERCENTILE(duration, 95) FROM traces",
        SqlDialect::DuckDB,
    );
    assert!(sql.contains("approx_quantile("), "got: {sql}");
    assert!(sql.contains("0.95"), "got: {sql}");
}

// ── Gated aggregation functions ─────────────────────────────────────────────

#[test]
fn node_status_gated() {
    let err = compile_err("SELECT NODE_STATUS() FROM metrics", SqlDialect::PostgreSQL);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "NODE_STATUS()"),
        "got: {err:?}"
    );
    if let ROSQLError::NotImplemented { message, .. } = err {
        assert!(message.contains("heartbeat"), "got: {message}");
    }
}

#[test]
fn expected_gated() {
    let err = compile_err("SELECT EXPECTED() FROM metrics", SqlDialect::PostgreSQL);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "EXPECTED()"),
        "got: {err:?}"
    );
}

#[test]
fn uptime_gated() {
    let err = compile_err("SELECT UPTIME() FROM metrics", SqlDialect::PostgreSQL);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "UPTIME()"),
        "got: {err:?}"
    );
}

#[test]
fn rate_gated() {
    let err = compile_err("SELECT RATE(duration) FROM traces", SqlDialect::PostgreSQL);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "RATE()"),
        "got: {err:?}"
    );
}

#[test]
fn delta_gated() {
    let err = compile_err(
        "SELECT DELTA(metric_value) FROM metrics",
        SqlDialect::PostgreSQL,
    );
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "DELTA()"),
        "got: {err:?}"
    );
}

// ── Gated compound clauses ───────────────────────────────────────────────────

#[test]
fn health_gated() {
    let err = compile_err("HEALTH()", SqlDialect::DuckDB);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "HEALTH()"),
        "got: {err:?}"
    );
}

#[test]
fn anomaly_gated() {
    let err = compile_err("ANOMALY(duration)", SqlDialect::DuckDB);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "ANOMALY()"),
        "got: {err:?}"
    );
}

#[test]
fn path_deviation_gated() {
    let err = compile_err("PATH DEVIATION", SqlDialect::DuckDB);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "PATH DEVIATION"),
        "got: {err:?}"
    );
}

#[test]
fn correlate_gated() {
    let err = compile_err("CORRELATE WITH metrics", SqlDialect::DuckDB);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "CORRELATE WITH"),
        "got: {err:?}"
    );
}

#[test]
fn show_recording_gated() {
    let err = compile_err("SHOW RECORDING", SqlDialect::DuckDB);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "SHOW RECORDING"),
        "got: {err:?}"
    );
}

#[test]
fn show_trace_breakdown_gated() {
    let err = compile_err("SHOW TRACE_BREAKDOWN", SqlDialect::DuckDB);
    assert!(
        matches!(&err, ROSQLError::NotImplemented { feature, .. } if feature == "SHOW TRACE_BREAKDOWN"),
        "got: {err:?}"
    );
}

// ── Default LIMIT ────────────────────────────────────────────────────────────

#[test]
fn default_limit_applied_for_basic_query() {
    let (sql, applied) = compile_with_default_limit("FROM logs", 100);
    assert!(applied, "expected default_limit_applied=true");
    assert!(sql.contains("LIMIT 100"), "got: {sql}");
}

#[test]
fn default_limit_not_applied_when_explicit_limit() {
    let (sql, applied) = compile_with_default_limit("FROM logs LIMIT 50", 100);
    assert!(!applied, "expected default_limit_applied=false");
    assert!(sql.contains("LIMIT 50"), "got: {sql}");
    assert!(!sql.contains("LIMIT 100"), "got: {sql}");
}

#[test]
fn default_limit_not_applied_for_scalar_aggregation() {
    let (sql, applied) = compile_with_default_limit("SELECT COUNT(*) FROM logs", 100);
    assert!(
        !applied,
        "scalar agg should be exempt, got applied={applied}"
    );
    assert!(
        !sql.contains("LIMIT"),
        "scalar agg should have no LIMIT, got: {sql}"
    );
}

#[test]
fn default_limit_not_applied_for_facet_query() {
    let (sql, applied) = compile_with_default_limit("FROM logs FACET severity", 100);
    assert!(
        !applied,
        "FACET query should be exempt, got applied={applied}"
    );
    assert!(
        !sql.contains("LIMIT"),
        "FACET query should have no LIMIT, got: {sql}"
    );
}

#[test]
fn default_limit_not_applied_for_trace_clause() {
    let (sql, applied) = compile_with_default_limit("TRACE 'abc123'", 100);
    assert!(!applied, "TRACE clause should be exempt");
    assert!(
        !sql.contains("LIMIT"),
        "TRACE should have no LIMIT, got: {sql}"
    );
}

#[test]
fn default_limit_not_applied_for_message_journey() {
    let (_, applied) = compile_with_default_limit("MESSAGE JOURNEY FOR TRACE 'abc'", 100);
    assert!(!applied, "MESSAGE JOURNEY should be exempt");
}

// ── OFFSET ────────────────────────────────────────────────────────────────────

#[test]
fn offset_compiles() {
    let sql = compile_sql("FROM logs LIMIT 20 OFFSET 40", SqlDialect::PostgreSQL);
    assert!(sql.contains("LIMIT 20"), "got: {sql}");
    assert!(sql.contains("OFFSET 40"), "got: {sql}");
}

#[test]
fn offset_only_compiles() {
    let sql = compile_sql("FROM logs OFFSET 10", SqlDialect::PostgreSQL);
    assert!(sql.contains("OFFSET 10"), "got: {sql}");
}

#[test]
fn offset_compiles_duckdb() {
    let sql = compile_sql("FROM logs LIMIT 10 OFFSET 5", SqlDialect::DuckDB);
    assert!(sql.contains("LIMIT 10"), "got: {sql}");
    assert!(sql.contains("OFFSET 5"), "got: {sql}");
}

// ── ALERT / DEFINE reserved keyword errors ───────────────────────────────────

#[test]
fn alert_reserved_keyword_message() {
    let errs = rosql::parse("ALERT WHEN cpu > 90").unwrap_err();
    assert!(matches!(&errs[0], ROSQLError::ReservedSyntax { keyword, .. } if keyword == "ALERT"));
    if let ROSQLError::ReservedSyntax { message, .. } = &errs[0] {
        assert!(message.contains("Robot Ops platform"), "got: {message}");
        assert!(message.contains("read-only"), "got: {message}");
    }
}

#[test]
fn define_reserved_keyword_message() {
    let errs = rosql::parse("DEFINE SLO availability 99.9").unwrap_err();
    assert!(matches!(&errs[0], ROSQLError::ReservedSyntax { keyword, .. } if keyword == "DEFINE"));
    if let ROSQLError::ReservedSyntax { message, .. } = &errs[0] {
        assert!(
            message.contains("Robot Ops platform dashboard"),
            "got: {message}"
        );
    }
}
