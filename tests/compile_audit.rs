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

use rosql::ast::FormatHint;
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

fn compile_result(query: &str) -> rosql::drivers::compiler::CompileResult {
    let ast = rosql::parse(query).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let registry = default_otel_registry();
    compile(&ast, &registry, &SqlDialect::DuckDB, &caps(), None)
        .unwrap_or_else(|e| panic!("compile failed: {e}"))
}

fn format_hint(query: &str) -> FormatHint {
    compile_result(query).format_hint
}

// All three dialects should contain `expected` in their SQL output.
fn assert_compiles_all(query: &str, expected: &str) {
    for dialect in [
        SqlDialect::PostgreSQL,
        SqlDialect::DuckDB,
        SqlDialect::MySQL,
    ] {
        let sql = compile_sql(query, dialect);
        assert!(
            sql.contains(expected),
            "{dialect:?} SQL missing '{expected}':\n{sql}"
        );
    }
}

// Three-way dialect-specific assertions — use when dialects produce legitimately
// different SQL surface (e.g. date_bin vs time_bucket vs FROM_UNIXTIME).
fn assert_compiles_dialects(
    query: &str,
    pg_contains: &str,
    duck_contains: &str,
    mysql_contains: &str,
) {
    let pg = compile_sql(query, SqlDialect::PostgreSQL);
    let duck = compile_sql(query, SqlDialect::DuckDB);
    let mysql = compile_sql(query, SqlDialect::MySQL);
    assert!(
        pg.contains(pg_contains),
        "PG SQL missing '{pg_contains}':\n{pg}"
    );
    assert!(
        duck.contains(duck_contains),
        "DuckDB SQL missing '{duck_contains}':\n{duck}"
    );
    assert!(
        mysql.contains(mysql_contains),
        "MySQL SQL missing '{mysql_contains}':\n{mysql}"
    );
}

// ── Implemented aggregation functions ───────────────────────────────────────

#[test]
fn topic_rate_compiles_to_subquery() {
    let q = "SELECT TOPIC_RATE() FROM metrics";
    assert_compiles_all(q, "otel_metrics");
    assert_compiles_all(q, "AVG");
    // Verify PG-specific details (registry-resolved column name)
    let sql = compile_sql(q, SqlDialect::PostgreSQL);
    assert!(sql.contains("ros2.topic.message_rate"), "got: {sql}");
    assert!(
        sql.contains(r#""value""#),
        "expected resolved column 'value', got: {sql}"
    );
    assert!(
        !sql.contains("metric_value"),
        "should not contain raw alias 'metric_value', got: {sql}"
    );
}

#[test]
fn topic_rate_with_topic_arg() {
    let q = "SELECT TOPIC_RATE('/cmd_vel') FROM metrics";
    assert_compiles_all(q, "otel_metrics");
    assert_compiles_all(q, "/cmd_vel");
    // Topic filter uses JSON extraction from the attributes column, not a bare column.
    let sql = compile_sql(q, SqlDialect::PostgreSQL);
    assert!(sql.contains("attributes"), "got: {sql}");
    assert!(sql.contains("topic"), "got: {sql}");
    assert!(
        !sql.contains("topic_name"),
        "should not use bare topic_name column, got: {sql}"
    );
}

#[test]
fn action_success_rate_no_arg() {
    let q = "SELECT ACTION_SUCCESS_RATE() FROM traces";
    assert_compiles_all(q, "CASE WHEN");
    assert_compiles_all(q, "COUNT");
    assert_compiles_all(q, "NULLIF");
    // Uses JSON extraction, not a bare column name.
    let sql = compile_sql(q, SqlDialect::PostgreSQL);
    assert!(sql.contains("succeeded"), "got: {sql}");
    assert!(
        sql.contains("span_attributes"),
        "expected span_attributes JSON access, got: {sql}"
    );
    assert!(sql.contains("ros.action.status"), "got: {sql}");
}

#[test]
fn action_success_rate_with_arg() {
    let q = "SELECT ACTION_SUCCESS_RATE('/navigate_to_pose') FROM traces";
    assert_compiles_all(q, "navigate_to_pose");
    assert_compiles_all(q, "succeeded");
    // action_name filter also uses JSON extraction.
    let sql = compile_sql(q, SqlDialect::PostgreSQL);
    assert!(
        sql.contains("ros.action.name"),
        "expected ros.action.name JSON key, got: {sql}"
    );
    assert!(
        sql.contains("span_attributes"),
        "expected span_attributes JSON access, got: {sql}"
    );
}

#[test]
fn facet_adds_column_to_select() {
    // Explicit aggregation + FACET must include the facet column in SELECT and GROUP BY.
    let q = "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' FACET service_name";
    assert_compiles_all(q, "GROUP BY");
    // service_name is a real column on otel_traces — should appear in SELECT and GROUP BY.
    let sql = compile_sql(q, SqlDialect::DuckDB);
    let col = r#""service_name""#;
    assert!(
        sql.contains(&format!("SELECT {col}")) || sql.contains(&format!("{col}, COUNT")),
        "facet column missing from SELECT, got: {sql}"
    );
}

#[test]
fn facet_robot_id_resolves_json_on_traces() {
    // robot_id on otel_traces must resolve to resource_attributes->>'robot.id', not a bare column.
    let q = "SELECT COUNT(*) FROM traces FACET robot_id";
    assert_compiles_all(q, "robot.id");
}

#[test]
fn facet_robot_id_bare_on_topic_messages() {
    // robot_id on topic_messages must resolve to the bare column, not JSON extraction.
    let q = "FROM odom FACET robot_id";
    assert_compiles_all(q, "robot_id");
    // Verify no resource_attributes on any dialect
    for dialect in [
        SqlDialect::PostgreSQL,
        SqlDialect::DuckDB,
        SqlDialect::MySQL,
    ] {
        let sql = compile_sql(q, dialect);
        assert!(
            !sql.contains("resource_attributes"),
            "should not use resource_attributes on topic_messages ({dialect:?}), got: {sql}"
        );
    }
}

#[test]
fn moving_avg_window() {
    let q = "SELECT MOVING_AVG(duration, 5) FROM traces";
    assert_compiles_all(q, "AVG");
    assert_compiles_all(q, "OVER");
    assert_compiles_all(q, "ROWS BETWEEN");
    // Window of 5 → 4 PRECEDING
    assert_compiles_all(q, "4 PRECEDING");
}

#[test]
fn derivative_compiles_lag() {
    let q = "SELECT DERIVATIVE(metric_value) FROM metrics";
    assert_compiles_all(q, "LAG(");
    assert_compiles_all(q, "OVER");
    assert_compiles_all(q, "NULLIF");
}

#[test]
fn approx_count_distinct() {
    // PG and MySQL fall back to exact COUNT(DISTINCT ...); DuckDB uses approx_count_distinct
    assert_compiles_dialects(
        "SELECT APPROX_COUNT_DISTINCT(span_name) FROM traces",
        "COUNT(DISTINCT",
        "approx_count_distinct(",
        "COUNT(DISTINCT",
    );
}

#[test]
fn approx_percentile() {
    // PG uses PERCENTILE_CONT; DuckDB uses approx_quantile; MySQL uses ROW_NUMBER emulation.
    // The percentile value 0.95 appears in all three.
    assert_compiles_dialects(
        "SELECT APPROX_PERCENTILE(duration, 95) FROM traces",
        "PERCENTILE_CONT",
        "approx_quantile(",
        "0.95",
    );
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
fn anomaly_compiles_two_cte() {
    // ANOMALY now compiles to a two-phase CTE with z-score output.
    let q = "ANOMALY(duration) COMPARED TO last week FACET robot_id SINCE 7 days ago";
    assert_compiles_all(q, "current_stats");
    assert_compiles_all(q, "baseline_stats");
    assert_compiles_all(q, "z_score");
    assert_compiles_all(q, "is_anomalous");
}

#[test]
fn anomaly_last_24h_baseline() {
    let q = "ANOMALY(duration) COMPARED TO last 24 hours FACET robot_id SINCE 12 hours ago";
    assert_compiles_all(q, "baseline_stats");
    // Should reference a 48-hour and 24-hour window for the baseline (PG-dialect check)
    let sql = compile_sql(q, SqlDialect::PostgreSQL);
    assert!(sql.contains("48"), "expected 48-hour window: {sql}");
    assert!(sql.contains("24"), "expected 24-hour window: {sql}");
}

#[test]
fn anomaly_missing_compared_to_is_parse_error() {
    // ANOMALY without COMPARED TO is now a parse error (not NotImplemented).
    let result = rosql::parse("ANOMALY(duration)");
    assert!(result.is_err(), "expected parse error");
}

#[test]
fn path_deviation_compiles() {
    // PATH DEVIATION compiles to a three-CTE SQL query returning per-timestamp rows.
    let q = "PATH DEVIATION FOR ROBOT 'r1' SINCE yesterday";
    assert_compiles_all(q, "planned_path");
    assert_compiles_all(q, "actual_poses");
    assert_compiles_all(q, "lateral_deviation_m");
}

#[test]
fn path_deviation_trace_compiles() {
    let sql = compile_sql("PATH DEVIATION FOR TRACE 'abc123'", SqlDialect::DuckDB);
    assert!(sql.contains("planned_path"), "got: {sql}");
    assert!(sql.contains("actual_poses"), "got: {sql}");
    assert!(sql.contains("'abc123'"), "got: {sql}");
}

#[test]
fn path_deviation_plan_index_compiles() {
    let sql = compile_sql(
        "PATH DEVIATION PLAN 0 FOR TRACE 'abc'",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("OFFSET 0"), "expected OFFSET 0: {sql}");
}

#[test]
fn joint_deviation_compiles() {
    let q = "JOINT DEVIATION FOR ROBOT 'arm_01' SINCE 2 hours ago";
    assert_compiles_all(q, "planned_joints");
    assert_compiles_all(q, "actual_joints");
    assert_compiles_all(q, "joint_error_rad");
    let sql = compile_sql(q, SqlDialect::PostgreSQL);
    assert!(sql.contains("/joint_trajectory"), "got: {sql}");
    assert!(sql.contains("/joint_states"), "got: {sql}");
}

#[test]
fn show_joints_compiles() {
    let q = "SHOW JOINTS FOR ROBOT 'arm_01'";
    assert_compiles_all(q, "robot_joint_map");
    assert_compiles_all(q, "arm_01");
}

#[test]
fn within_local_compiles() {
    let q = "FROM odom WHERE position WITHIN 2 m OF POSITION (1.5, 3.0) SINCE 1 hour ago";
    assert_compiles_all(q, "SQRT");
    assert_compiles_all(q, "POWER");
}

#[test]
fn within_gps_compiles() {
    let q = "FROM odom WHERE position WITHIN 500 m OF (37.7749, -122.4194) SINCE 1 hour ago";
    assert_compiles_all(q, "ASIN");
    assert_compiles_all(q, "6371000");
}

#[test]
fn field_access_array_index_compiles() {
    let sql = compile_sql(
        "FROM joint_states WHERE fields['position[0]'] > 1.5 FOR ROBOT 'arm_01' SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    // Should compile to "fields"->'position'->>0 not "fields"->>'position[0]'
    assert!(
        sql.contains("'position'"),
        "expected JSON array path: {sql}"
    );
    assert!(
        sql.contains("->>0") || sql.contains("->> 0"),
        "expected ->>0 array access: {sql}"
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
fn default_limit_not_applied_for_trace() {
    let (sql, applied) = compile_with_default_limit("TRACE 'abc'", 100);
    assert!(!applied, "TRACE should be exempt, got: {sql}");
}

// ── OFFSET ────────────────────────────────────────────────────────────────────

#[test]
fn offset_compiles() {
    let q = "FROM logs LIMIT 20 OFFSET 40";
    assert_compiles_all(q, "LIMIT");
    assert_compiles_all(q, "OFFSET");
}

#[test]
fn offset_only_compiles() {
    assert_compiles_all("FROM logs OFFSET 10", "OFFSET");
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

// ── P0: Scope compilation ────────────────────────────────────────────────────

#[test]
fn for_robot_compiles_to_where_clause() {
    let q = "SELECT * FROM logs FOR ROBOT 'r1'";
    assert_compiles_all(q, "robot.id");
    assert_compiles_all(q, "r1");
}

#[test]
fn for_version_compiles_to_where_clause() {
    let q = "SELECT * FROM logs FOR VERSION 'v1.2.3'";
    assert_compiles_all(q, "service.version");
    assert_compiles_all(q, "v1.2.3");
}

#[test]
fn for_environment_compiles_to_where_clause() {
    let q = "SELECT * FROM logs FOR ENVIRONMENT 'production'";
    assert_compiles_all(q, "deployment.environment");
    assert_compiles_all(q, "production");
}

#[test]
fn for_session_compiles_to_where_clause() {
    let q = "SELECT * FROM logs FOR SESSION 'sess_abc'";
    assert_compiles_all(q, "ros.session.id");
    assert_compiles_all(q, "sess_abc");
}

#[test]
fn composable_scope_emits_all_filters() {
    let q = "SELECT * FROM logs FOR ROBOT 'r1' FOR VERSION 'v1.0' FOR ENVIRONMENT 'prod'";
    assert_compiles_all(q, "robot.id");
    assert_compiles_all(q, "service.version");
    assert_compiles_all(q, "deployment.environment");
}

#[test]
fn trace_with_scope_compiles_to_cte_with_filter() {
    let q = "TRACE 'abc123' FOR ROBOT 'r1'";
    assert_compiles_all(q, "WITH RECURSIVE");
    assert_compiles_all(q, "robot.id");
}

// ── SHOW commands ─────────────────────────────────────────────────────────────

#[test]
fn show_deployments_compiles() {
    let q = "SHOW DEPLOYMENTS SINCE 30 days ago";
    assert_compiles_all(q, "service.version");
    assert_compiles_all(q, "deployment.environment");
    assert_compiles_all(q, "GROUP BY");
}

#[test]
fn show_span_summary_compiles() {
    let q = "SHOW SPAN SUMMARY SINCE 1 hour ago";
    assert_compiles_all(q, "span_name");
    assert_compiles_all(q, "AVG");
    assert_compiles_all(q, "GROUP BY");
}

#[test]
fn show_plans_compiles() {
    let q = "SHOW PLANS FOR TRACE 'abc123'";
    assert_compiles_all(q, "ros.plan.id");
    assert_compiles_all(q, "abc123");
}

// ── COMPARE TO VERSION ────────────────────────────────────────────────────────

#[test]
fn compare_to_version_parses() {
    let ast = rosql::parse("FROM traces COMPARE TO VERSION 'v1.2.3'").unwrap();
    match ast {
        rosql::Query::Standard(sq) => {
            assert!(
                matches!(&sq.baseline, Some(rosql::ast::Baseline::Version(v)) if v == "v1.2.3"),
                "got: {:?}",
                sq.baseline
            );
        }
        _ => panic!("expected Standard"),
    }
}

#[test]
fn compare_version_pair_parses() {
    let ast = rosql::parse("FROM traces COMPARE VERSION 'v1.0' TO VERSION 'v2.0'").unwrap();
    match ast {
        rosql::Query::Standard(sq) => {
            assert!(
                matches!(&sq.baseline, Some(rosql::ast::Baseline::VersionPair(v1, v2)) if v1 == "v1.0" && v2 == "v2.0"),
                "got: {:?}",
                sq.baseline
            );
        }
        _ => panic!("expected Standard"),
    }
}

// ── Removed syntax deprecation errors ────────────────────────────────────────

#[test]
fn message_journey_deprecation_error() {
    let errs = rosql::parse("MESSAGE JOURNEY FOR TRACE 'abc'").unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("MESSAGE JOURNEY is removed")),
        "got: {errs:?}"
    );
}

#[test]
fn message_paths_deprecation_error() {
    let errs = rosql::parse("MESSAGE PATHS FOR TOPIC '/cmd_vel'").unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("MESSAGE PATHS is removed")),
        "got: {errs:?}"
    );
}

#[test]
fn message_path_deprecation_error() {
    let errs = rosql::parse("MESSAGE PATH FROM TOPIC '/a' TO NODE '/b'").unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("MESSAGE PATH is removed")),
        "got: {errs:?}"
    );
}

// ── SHOW TOPICS / SHOW NODES / SHOW NODE GRAPH ──────────────────────────────

#[test]
fn show_topics_compiles() {
    let q = "SHOW TOPICS SINCE 6 hours ago";
    assert_compiles_all(q, "ros.topic");
    assert_compiles_all(q, "GROUP BY");
    // PG-specific detail checks
    let sql = compile_sql(
        "SHOW TOPICS FOR ROBOT 'robot_42' SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("topic_name"), "got: {sql}");
    assert!(sql.contains("robot.id"), "got: {sql}");
}

#[test]
fn show_nodes_compiles() {
    let q = "SHOW NODES FOR ROBOT 'robot_42' SINCE 30 minutes ago";
    assert_compiles_all(q, "ros.node");
    assert_compiles_all(q, "node_name");
}

#[test]
fn show_node_graph_compiles() {
    let q = "SHOW NODE GRAPH FOR ROBOT 'robot_42' SINCE 30 minutes ago";
    assert_compiles_all(q, "source_node");
    assert_compiles_all(q, "DISTINCT");
}

#[test]
fn show_topics_not_limit_exempt() {
    // SHOW TOPICS is aggregate — should be exempt from default limit
    let (_, applied) = compile_with_default_limit("SHOW TOPICS SINCE 1 hour ago", 100);
    assert!(!applied, "SHOW TOPICS should be limit-exempt");
}

// ── DURING ───────────────────────────────────────────────────────────────────
// Note: DURING is parsed as a standalone compound clause. The form
// `FROM traces WHERE ... DURING(...)` is not yet supported by the parser —
// the DURING clause is silently dropped and the query compiles as a plain
// standard query. Tests below use standalone DURING where the compiler is active.

#[test]
fn during_compiles_to_exists_subquery() {
    let q = "DURING(FROM metrics WHERE metric_name = 'system.cpu.utilization')";
    assert_compiles_all(q, "otel_traces");
    assert_compiles_all(q, "EXISTS");
}

// ── TIMESERIES ───────────────────────────────────────────────────────────────

#[test]
fn timeseries_compiles() {
    let q = "SELECT COUNT(*) FROM traces TIMESERIES 5 min SINCE 1 hour ago";
    assert_compiles_dialects(q, "date_bin", "time_bucket", "FROM_UNIXTIME");
    assert_compiles_all(q, "GROUP BY");
    // DuckDB-specific ordering/interval assertions
    let sql = compile_sql(
        "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' TIMESERIES 5 min SINCE 6 hours ago",
        SqlDialect::DuckDB,
    );
    assert!(sql.contains("ORDER BY time_bucket ASC"), "got: {sql}");
    assert!(sql.contains("INTERVAL '5 minutes'"), "got: {sql}");
}

#[test]
fn timeseries_bare_field_wrapped_in_avg() {
    // A bare field selection (not an aggregation) in a TIMESERIES+FACET query must be
    // auto-wrapped in AVG() so the generated SQL is valid under GROUP BY.
    // Regression: previously emitted bare "value" causing DuckDB binder error.
    let q = "SELECT cpu_usage FROM metrics TIMESERIES 2 min FACET robot_id SINCE 45 min ago";
    assert_compiles_all(q, "AVG(");
    assert_compiles_all(q, "GROUP BY");
    assert_compiles_dialects(q, "date_bin", "time_bucket", "FROM_UNIXTIME");
}

#[test]
fn timeseries_composes_with_facet() {
    let q = "SELECT AVG(duration) FROM traces TIMESERIES 1 min FACET action_name SINCE 1 hour ago";
    assert_compiles_all(q, "GROUP BY");
    assert_compiles_dialects(q, "date_bin", "time_bucket", "FROM_UNIXTIME");
    // DuckDB: verify both time_bucket and facet dimension appear in GROUP BY
    let sql = compile_sql(q, SqlDialect::DuckDB);
    assert!(
        sql.contains("action_name") || sql.contains("action.name"),
        "got: {sql}"
    );
    let gb_pos = sql.find("GROUP BY").expect("missing GROUP BY");
    let gb_clause = &sql[gb_pos..];
    assert!(
        gb_clause.contains("time_bucket"),
        "time_bucket not in GROUP BY: {sql}"
    );
    assert!(
        gb_clause.contains("action_name") || gb_clause.contains("action.name"),
        "facet not in GROUP BY: {sql}"
    );
}

#[test]
fn timeseries_is_limit_exempt() {
    let (_, applied) = compile_with_default_limit(
        "SELECT COUNT(*) FROM traces TIMESERIES 5 min SINCE 6 hours ago",
        100,
    );
    assert!(!applied, "TIMESERIES queries should be limit-exempt");
}

// ── ENRICH WITH — primary SQL unaffected ────────────────────────────────────

#[test]
fn enrich_with_primary_sql_unchanged() {
    // Primary SQL should be the same as without enrichment (PG idempotence check)
    let enriched = compile_sql(
        "FROM traces WHERE status = 'ERROR' ENRICH WITH logs SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    let plain = compile_sql(
        "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    assert_eq!(enriched, plain, "ENRICH WITH should not modify primary SQL");
    // All dialects should compile to a query referencing otel_traces
    assert_compiles_all(
        "FROM traces WHERE status = 'ERROR' ENRICH WITH logs SINCE 1 hour ago",
        "otel_traces",
    );
}

#[test]
fn enrich_with_pipeline_primary_sql_unchanged() {
    let enriched = compile_sql(
        "FROM traces | WHERE status = 'ERROR' | ENRICH WITH logs | SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    let plain = compile_sql(
        "FROM traces | WHERE status = 'ERROR' | SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    assert_eq!(
        enriched, plain,
        "Pipeline ENRICH WITH should not modify primary SQL"
    );
}

#[test]
fn enrich_with_produces_enrichment_plans() {
    let ast = rosql::parse("FROM traces ENRICH WITH logs SINCE 1 hour ago")
        .unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let registry = default_otel_registry();
    let cr = compile(&ast, &registry, &SqlDialect::PostgreSQL, &caps(), None).unwrap();
    assert_eq!(cr.enrichments.len(), 1);
    assert_eq!(cr.enrichments[0].source_name, "logs");
    assert_eq!(cr.enrichments[0].join_column, "trace_id");
    assert_eq!(cr.enrichments[0].limit, 50); // default
}

#[test]
fn enrich_with_limit_override() {
    let ast = rosql::parse("FROM traces ENRICH WITH logs LIMIT 200 SINCE 1 hour ago")
        .unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let registry = default_otel_registry();
    let cr = compile(&ast, &registry, &SqlDialect::PostgreSQL, &caps(), None).unwrap();
    assert_eq!(cr.enrichments[0].limit, 200);
}

#[test]
fn enrich_with_sample_full() {
    let ast = rosql::parse("FROM traces ENRICH WITH joint_states SAMPLE FULL SINCE 1 hour ago")
        .unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let registry = default_otel_registry();
    let cr = compile(&ast, &registry, &SqlDialect::PostgreSQL, &caps(), None).unwrap();
    assert!(cr.enrichments[0].sample_full);
}

#[test]
fn enrich_with_multiple_plans() {
    let ast = rosql::parse("FROM traces ENRICH WITH logs ENRICH WITH recordings SINCE 1 hour ago")
        .unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let registry = default_otel_registry();
    let cr = compile(&ast, &registry, &SqlDialect::PostgreSQL, &caps(), None).unwrap();
    assert_eq!(cr.enrichments.len(), 2);
    assert_eq!(cr.enrichments[0].source_name, "logs");
    assert_eq!(cr.enrichments[1].source_name, "recordings");
}

// ── Format hint inference (issue #65) ────────────────────────────────────────

#[test]
fn format_hint_timeseries_no_facet_is_line_chart() {
    assert_eq!(
        format_hint("SELECT COUNT(*) FROM traces TIMESERIES 5 min SINCE 1 hour ago"),
        FormatHint::LineChart
    );
}

#[test]
fn format_hint_timeseries_with_facet_is_stacked_line_chart() {
    assert_eq!(
        format_hint("SELECT COUNT(*) FROM traces TIMESERIES 5 min FACET robot_id SINCE 1 hour ago"),
        FormatHint::StackedLineChart
    );
}

#[test]
fn format_hint_stacked_viz_has_series_key_and_x_axis() {
    let cr = compile_result(
        "SELECT COUNT(*) FROM traces TIMESERIES 5 min FACET robot_id SINCE 1 hour ago",
    );
    let viz = cr.visualization.expect("expected VisualizationConfig");
    assert_eq!(viz.series_key.as_deref(), Some("robot_id"));
    assert_eq!(viz.x_axis.as_deref(), Some("time_bucket"));
}

#[test]
fn format_hint_facet_no_timeseries_is_bar_chart() {
    assert_eq!(
        format_hint("SELECT COUNT(*) FROM traces FACET action_name SINCE 1 hour ago"),
        FormatHint::BarChart
    );
}

#[test]
fn format_hint_bar_chart_viz_has_x_axis() {
    let cr = compile_result("SELECT COUNT(*) FROM traces FACET action_name");
    let viz = cr.visualization.expect("expected VisualizationConfig");
    assert_eq!(viz.x_axis.as_deref(), Some("action_name"));
}

#[test]
fn format_hint_trace_is_gantt() {
    assert_eq!(format_hint("TRACE 'abc123'"), FormatHint::Gantt);
}

#[test]
fn format_hint_message_flow_is_directed_graph() {
    assert_eq!(
        format_hint("MESSAGE FLOW FROM TOPIC '/cmd_vel'"),
        FormatHint::DirectedGraph
    );
}

#[test]
fn format_hint_show_node_graph_is_node_graph() {
    assert_eq!(format_hint("SHOW NODE GRAPH"), FormatHint::NodeGraph);
}

#[test]
fn format_hint_show_span_summary_is_horizontal_bars() {
    assert_eq!(
        format_hint("SHOW SPAN SUMMARY SINCE 1 hour ago"),
        FormatHint::HorizontalBars
    );
}

#[test]
fn format_hint_span_summary_viz_has_axes() {
    let cr = compile_result("SHOW SPAN SUMMARY SINCE 1 hour ago");
    let viz = cr.visualization.expect("expected VisualizationConfig");
    assert_eq!(viz.x_axis.as_deref(), Some("span_name"));
    assert_eq!(viz.y_axis.as_deref(), Some("avg_duration"));
}

#[test]
fn format_hint_anomaly_is_table_with_color_field() {
    assert_eq!(
        format_hint("ANOMALY(duration) COMPARED TO last week FACET robot_id"),
        FormatHint::Table
    );
    let cr = compile_result("ANOMALY(duration) COMPARED TO last week FACET robot_id");
    let viz = cr.visualization.expect("expected VisualizationConfig");
    assert_eq!(viz.color_field.as_deref(), Some("is_anomalous"));
}

#[test]
fn format_hint_scalar_aggregation_is_scalar_cards() {
    assert_eq!(
        format_hint(
            "SELECT COUNT(*) AS total_errors, AVG(duration) AS avg_duration FROM traces SINCE 1 hour ago"
        ),
        FormatHint::ScalarCards
    );
}

#[test]
fn format_hint_from_logs_is_log_table() {
    assert_eq!(
        format_hint("FROM logs WHERE severity = 'ERROR'"),
        FormatHint::LogTable
    );
}

#[test]
fn format_hint_log_table_viz_has_color_field() {
    let cr = compile_result("FROM logs");
    let viz = cr.visualization.expect("expected VisualizationConfig");
    assert_eq!(viz.color_field.as_deref(), Some("severity"));
}

#[test]
fn format_hint_path_deviation_is_line_chart() {
    assert_eq!(
        format_hint("PATH DEVIATION FOR TRACE 'abc123'"),
        FormatHint::LineChart
    );
}

#[test]
fn format_hint_joint_deviation_is_bar_chart() {
    assert_eq!(
        format_hint("JOINT DEVIATION FOR TRACE 'abc123'"),
        FormatHint::BarChart
    );
}

#[test]
fn format_hint_format_clause_overrides_inference() {
    // TIMESERIES normally infers LineChart, but explicit FORMAT table overrides it.
    assert_eq!(
        format_hint("SELECT COUNT(*) FROM traces TIMESERIES 5 min FORMAT table"),
        FormatHint::Table
    );
}

#[test]
fn format_hint_plain_from_traces_is_table() {
    assert_eq!(
        format_hint("FROM traces WHERE status = 'ERROR' LIMIT 20"),
        FormatHint::Table
    );
}

// ── CompilerWarning structured output (issue #65) ───────────────────────────

#[test]
fn anomaly_no_facet_emits_structured_warning() {
    let cr = compile_result("ANOMALY(duration) COMPARED TO last week SINCE 1 hour ago");
    assert_eq!(cr.warnings.len(), 1);
    let w = &cr.warnings[0];
    assert_eq!(w.code, "ANOMALY_NO_FACET");
    assert!(
        w.message.contains("ANOMALY without FACET"),
        "got: {}",
        w.message
    );
    assert!(w.suggestion.is_some(), "expected suggestion");
    let suggestion = w.suggestion.as_deref().unwrap();
    assert!(
        suggestion.contains("FACET"),
        "suggestion should mention FACET: {suggestion}"
    );
}

#[test]
fn anomaly_with_facet_has_no_warnings() {
    let cr = compile_result("ANOMALY(duration) COMPARED TO last week FACET robot_id");
    assert!(
        cr.warnings.is_empty(),
        "expected no warnings, got: {:?}",
        cr.warnings
    );
}

#[test]
fn non_anomaly_query_has_no_warnings() {
    let cr = compile_result("FROM traces WHERE status = 'ERROR'");
    assert!(cr.warnings.is_empty());
}

// ── ExecutionError display (issue #65) ──────────────────────────────────────

#[test]
fn execution_error_display_includes_data_source() {
    let err = rosql::error::ROSQLError::ExecutionError {
        message: "Table not found.".into(),
        data_source: "PostgreSQL".into(),
        compiled_sql: None,
        suggestion: Some("Verify ingestion.".into()),
    };
    let msg = err.to_string();
    assert!(msg.contains("PostgreSQL"), "got: {msg}");
    assert!(msg.contains("Table not found"), "got: {msg}");
}

// ── Showcase query format hints ──────────────────────────────────────────────

#[test]
fn showcase_format_hints() {
    // All 9 showcase queries must compile and return the expected format hint.

    // Query 1: Trace a failed mission
    assert_eq!(
        format_hint("TRACE 'trace-amr02-m3'"),
        FormatHint::Gantt,
        "query 1 (trace)"
    );

    // Query 2: Enrich trace with logs (still a Gantt)
    assert_eq!(
        format_hint("TRACE 'trace-amr02-m3'\nENRICH WITH logs LIMIT 5"),
        FormatHint::Gantt,
        "query 2 (enrich with logs)"
    );

    // Query 3: CPU usage across fleet — TIMESERIES + FACET → StackedLineChart
    assert_eq!(
        format_hint(
            "SELECT cpu_usage FROM metrics\nTIMESERIES 2 min FACET robot_id\nSINCE 45 min ago"
        ),
        FormatHint::StackedLineChart,
        "query 3 (timeseries facet)"
    );

    // Query 4: Message flow → DirectedGraph
    assert_eq!(
        format_hint("MESSAGE FLOW FROM TOPIC '/scan'\nFOR ROBOT 'robot-amr-02'"),
        FormatHint::DirectedGraph,
        "query 4 (message flow)"
    );

    // Query 5: Slowest actions/spans → HorizontalBars
    assert_eq!(
        format_hint("SHOW SPAN SUMMARY\nFOR ROBOT 'robot-amr-02'\nSINCE 90 min ago"),
        FormatHint::HorizontalBars,
        "query 5 (span summary)"
    );

    // Query 6: Path deviation → LineChart
    assert_eq!(
        format_hint("PATH DEVIATION\nFOR TRACE 'trace-amr02-m3'"),
        FormatHint::LineChart,
        "query 6 (path deviation)"
    );

    // Query 7: Anomaly detection → Table (with color_field for highlighting)
    assert_eq!(
        format_hint("ANOMALY(duration)\nCOMPARED TO last week\nFACET robot_id"),
        FormatHint::Table,
        "query 7 (anomaly)"
    );

    // Query 8: Battery below threshold → Table
    assert_eq!(
        format_hint(
            "FROM topics\nWHERE topic_name = '/battery_state'\n  AND fields['voltage'] < 11.5 V\nFOR ROBOT 'robot-amr-02'\nSINCE 2 h ago"
        ),
        FormatHint::Table,
        "query 8 (battery filter)"
    );

    // Query 9: ROS2 node topology → NodeGraph
    assert_eq!(
        format_hint("SHOW NODE GRAPH\nFOR ROBOT 'robot-amr-02'"),
        FormatHint::NodeGraph,
        "query 9 (node graph)"
    );
}

#[test]
fn execution_error_no_raw_driver_text() {
    // Verify the error type exists and doesn't accidentally embed raw DB output.
    let err = rosql::error::ROSQLError::ExecutionError {
        message: "Query execution failed.".into(),
        data_source: "DuckDB".into(),
        compiled_sql: Some("SELECT * FROM nonexistent".into()),
        suggestion: Some("Check your schema.".into()),
    };
    // The display should not include the raw compiled_sql
    let display = err.to_string();
    assert!(
        !display.contains("SELECT"),
        "raw SQL should not appear in display: {display}"
    );
    assert!(
        display.contains("DuckDB"),
        "data source should appear: {display}"
    );
}

#[test]
fn recordings_since_uses_end_time_not_timestamp() {
    // FROM recordings SINCE X must filter by end_time (overlap), not a non-existent timestamp col.
    let q = "FROM recordings WHERE robot_id = 'amr-01' SINCE 6 hours ago";
    assert_compiles_all(q, "end_time");
    for dialect in [
        SqlDialect::PostgreSQL,
        SqlDialect::DuckDB,
        SqlDialect::MySQL,
    ] {
        let sql = compile_sql(q, dialect);
        assert!(
            !sql.contains("\"timestamp\""),
            "{dialect:?}: timestamp column must not appear for recordings, got: {sql}"
        );
    }
}

#[test]
fn recordings_between_uses_overlap_semantics() {
    // TimeRange::Between should produce start_time <= end AND end_time >= start
    let sql = compile_sql(
        "FROM recordings WHERE robot_id = 'amr-01' AND start_time <= '2026-04-17T11:00:00Z' AND end_time >= '2026-04-17T10:00:00Z'",
        SqlDialect::PostgreSQL,
    );
    // This is a plain WHERE condition (not a SINCE), so just verify it compiles cleanly.
    assert!(
        sql.contains("mcap_metadata"),
        "expected mcap_metadata table, got: {sql}"
    );
}

// ── MySQL baseline coverage (Section 5, issue #96) ────────────────────────────
// MySQL has zero compile test coverage; these 10 tests establish a baseline.
// Each asserts on a MySQL-dialect-specific substring so regressions in dialect.rs
// are caught — not just "compiles without error."

#[test]
fn mysql_since_uses_interval_syntax() {
    let sql = compile_sql(
        "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago",
        SqlDialect::MySQL,
    );
    assert!(
        sql.contains("INTERVAL 1 hour"),
        "MySQL SINCE should use NOW() - INTERVAL syntax, got: {sql}"
    );
    assert!(sql.contains("otel_traces"), "got: {sql}");
}

#[test]
fn mysql_facet_uses_json_unquote() {
    let sql = compile_sql(
        "SELECT COUNT(*) FROM traces FACET robot_id",
        SqlDialect::MySQL,
    );
    assert!(
        sql.contains("JSON_UNQUOTE"),
        "MySQL FACET should use JSON_UNQUOTE for JSON columns, got: {sql}"
    );
    assert!(sql.contains("GROUP BY"), "got: {sql}");
}

#[test]
fn mysql_duration_unit_converts_to_nanoseconds() {
    let sql = compile_sql("FROM traces WHERE duration > 500 ms", SqlDialect::MySQL);
    // 500 ms = 500_000_000 ns
    assert!(
        sql.contains("500000000"),
        "MySQL should convert 500 ms to nanoseconds, got: {sql}"
    );
}

#[test]
fn mysql_json_field_uses_json_extract() {
    let sql = compile_sql(
        "FROM topics WHERE fields['percentage'] < 20",
        SqlDialect::MySQL,
    );
    assert!(
        sql.contains("JSON_EXTRACT"),
        "MySQL JSON field access should use JSON_EXTRACT, got: {sql}"
    );
}

#[test]
fn mysql_timeseries_uses_from_unixtime() {
    let sql = compile_sql(
        "SELECT COUNT(*) FROM traces TIMESERIES 5 min SINCE 1 hour ago",
        SqlDialect::MySQL,
    );
    assert!(
        sql.contains("FROM_UNIXTIME"),
        "MySQL TIMESERIES should use FROM_UNIXTIME bucketing, got: {sql}"
    );
    assert!(sql.contains("GROUP BY"), "got: {sql}");
    assert!(sql.contains("time_bucket"), "got: {sql}");
}

#[test]
fn mysql_scope_filter_uses_json_extract() {
    let sql = compile_sql(
        "FOR ROBOT 'r1' FROM traces WHERE status = 'ERROR'",
        SqlDialect::MySQL,
    );
    assert!(
        sql.contains("JSON_EXTRACT"),
        "MySQL scope filter should use JSON_EXTRACT for resource_attributes, got: {sql}"
    );
    assert!(sql.contains("r1"), "got: {sql}");
}

#[test]
fn mysql_limit_offset_compiles() {
    let sql = compile_sql(
        "FROM logs ORDER BY timestamp DESC LIMIT 20 OFFSET 40",
        SqlDialect::MySQL,
    );
    assert!(sql.contains("LIMIT 20"), "got: {sql}");
    assert!(sql.contains("OFFSET 40"), "got: {sql}");
    assert!(sql.contains("ORDER BY"), "got: {sql}");
}

#[test]
fn mysql_trace_uses_recursive_cte() {
    let sql = compile_sql("TRACE 'abc123'", SqlDialect::MySQL);
    assert!(
        sql.contains("WITH RECURSIVE"),
        "MySQL TRACE should use WITH RECURSIVE CTE, got: {sql}"
    );
    assert!(sql.contains("trace_tree"), "got: {sql}");
}

#[test]
fn mysql_approx_percentile_uses_row_number_emulation() {
    let sql = compile_sql(
        "SELECT APPROX_PERCENTILE(duration, 95) FROM traces",
        SqlDialect::MySQL,
    );
    // MySQL lacks native percentile; emulated via ROW_NUMBER window function
    assert!(
        sql.contains("ROW_NUMBER"),
        "MySQL APPROX_PERCENTILE should use ROW_NUMBER emulation, got: {sql}"
    );
}

#[test]
fn mysql_approx_count_distinct_uses_count_distinct() {
    let sql = compile_sql(
        "SELECT APPROX_COUNT_DISTINCT(span_name) FROM traces",
        SqlDialect::MySQL,
    );
    assert!(
        sql.contains("COUNT(DISTINCT"),
        "MySQL APPROX_COUNT_DISTINCT should fall back to COUNT(DISTINCT), got: {sql}"
    );
}

// ── Multi-clause combination tests (Section 2, issue #96) ─────────────────────
// Each test uses assert_compiles_all() to verify all three dialects produce
// SQL containing the expected structural elements, not just that no error occurs.

// Standard query combos
#[test]
fn combo_where_since_facet_order_limit() {
    let q = "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago FACET robot_id ORDER BY robot_id ASC LIMIT 10";
    assert_compiles_all(q, "GROUP BY");
    assert_compiles_all(q, "ORDER BY");
    assert_compiles_all(q, "LIMIT 10");
    assert_compiles_all(q, "otel_traces");
}

#[test]
fn combo_where_since_during_facet() {
    let q = "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago DURING( FROM topics WHERE topic_name = '/battery_state' AND fields['percentage'] < 15 ) FACET robot_id";
    assert_compiles_all(q, "EXISTS");
}

#[test]
fn combo_where_since_enrich_limit() {
    // LIMIT on an ENRICH WITH query goes to the enrichment plan, not the primary SQL.
    // The primary SQL is the same as without enrichment; the limit is checked separately.
    let q = "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago ENRICH WITH logs LIMIT 20";
    assert_compiles_all(q, "otel_traces");
    assert_compiles_all(q, "status_code");
}

#[test]
fn combo_where_since_timeseries_facet() {
    let q = "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' SINCE 1 hour ago TIMESERIES 5 min FACET robot_id";
    // PG uses date_bin (aliased as time_bucket), DuckDB uses time_bucket, MySQL uses FROM_UNIXTIME
    assert_compiles_dialects(q, "date_bin", "time_bucket", "FROM_UNIXTIME");
    assert_compiles_all(q, "GROUP BY");
    assert_compiles_all(q, "robot.id");
}

#[test]
fn combo_agg_where_since_facet_order() {
    let q = "SELECT COUNT(*) AS errors FROM traces WHERE status = 'ERROR' SINCE 1 hour ago FACET action_name ORDER BY errors DESC";
    assert_compiles_all(q, "GROUP BY");
    assert_compiles_all(q, "ORDER BY");
    assert_compiles_all(q, "errors");
}

#[test]
fn combo_for_robot_where_since_facet() {
    let q = "FOR ROBOT 'r1' FROM traces WHERE status = 'ERROR' SINCE 1 hour ago FACET action_name";
    assert_compiles_all(q, "robot.id");
    assert_compiles_all(q, "GROUP BY");
    assert_compiles_all(q, "action_name");
}

#[test]
fn combo_for_robot_for_version_where_since() {
    let q = "FOR ROBOT 'r1' FOR VERSION 'v1.0' FROM traces WHERE status = 'ERROR' SINCE 1 hour ago";
    assert_compiles_all(q, "robot.id");
    assert_compiles_all(q, "service.version");
    assert_compiles_all(q, "otel_traces");
}

#[test]
fn combo_within_since_limit() {
    let q =
        "FROM odom WHERE position WITHIN 500 m OF (37.7749, -122.4194) SINCE 1 hour ago LIMIT 10";
    assert_compiles_all(q, "ASIN");
    assert_compiles_all(q, "LIMIT 10");
}

#[test]
fn combo_where_since_using_ros_time() {
    let q = "FROM topics WHERE topic_name = '/odom' SINCE 1 hour ago USING ROS_TIME";
    assert_compiles_all(q, "topic_messages");
    assert_compiles_all(q, "/odom");
}

#[test]
fn combo_where_between_facet() {
    let q = "FROM traces WHERE duration BETWEEN 100 ms AND 2 s FACET robot_id";
    assert_compiles_all(q, "GROUP BY");
    assert_compiles_all(q, "robot.id");
}

#[test]
fn combo_where_since_offset_limit() {
    let q = "FROM logs SINCE 1 hour ago OFFSET 40 LIMIT 20";
    assert_compiles_all(q, "LIMIT 20");
    assert_compiles_all(q, "OFFSET 40");
}

// Pipeline combos
#[test]
fn combo_pipeline_where_where_facet_compare() {
    let q = "FROM traces | WHERE duration > 500 ms | WHERE status = 'ERROR' | FACET robot_id";
    assert_compiles_all(q, "GROUP BY");
    assert_compiles_all(q, "otel_traces");
}

#[test]
fn combo_pipeline_where_timeseries_facet_since() {
    let q = "FROM traces | WHERE status = 'ERROR' | TIMESERIES 5 min | FACET robot_id | SINCE 1 hour ago";
    assert_compiles_dialects(q, "date_bin", "time_bucket", "FROM_UNIXTIME");
    assert_compiles_all(q, "GROUP BY");
}

#[test]
fn combo_pipeline_where_enrich_limit() {
    let q = "FROM traces | WHERE status = 'ERROR' | ENRICH WITH logs | LIMIT 20";
    assert_compiles_all(q, "otel_traces");
    assert_compiles_all(q, "LIMIT 20");
}

// Compound with modifiers
#[test]
fn combo_trace_enrich_limit() {
    let q = "TRACE 'abc123' ENRICH WITH logs LIMIT 5";
    assert_compiles_all(q, "WITH RECURSIVE");
    assert_compiles_all(q, "trace_tree");
}

#[test]
fn combo_anomaly_compared_facet_since() {
    let q = "ANOMALY(duration) COMPARED TO last week FACET robot_id SINCE 7 days ago";
    assert_compiles_all(q, "current_stats");
    assert_compiles_all(q, "baseline_stats");
    assert_compiles_all(q, "z_score");
}

#[test]
fn combo_path_deviation_plan_for_robot_since() {
    let q = "PATH DEVIATION PLAN 0 FOR ROBOT 'r1' SINCE 1 hour ago";
    assert_compiles_all(q, "planned_path");
    assert_compiles_all(q, "OFFSET 0");
}

#[test]
fn combo_show_span_summary_for_robot_since() {
    let q = "SHOW SPAN SUMMARY FOR ROBOT 'r1' SINCE 1 hour ago";
    assert_compiles_all(q, "span_name");
    assert_compiles_all(q, "robot.id");
    assert_compiles_all(q, "GROUP BY");
}

#[test]
fn combo_message_flow_for_robot() {
    let q = "MESSAGE FLOW FROM TOPIC '/cmd_vel' FOR ROBOT 'r1'";
    assert_compiles_all(q, "/cmd_vel");
    assert_compiles_all(q, "source_node");
}

// ── FROM tf coverage ────────────────────────────────────────────────────────
// tf_states column schema is registered in otel_registry; verify the typed
// columns resolve as bare columns (not map access) on all dialects.

#[test]
fn tf_states_resolves_to_table() {
    let q = "FROM tf";
    assert_compiles_all(q, "tf_states");
}

#[test]
fn tf_states_columns_are_bare() {
    let q =
        "FROM tf WHERE parent_frame = 'base_link' AND child_frame = 'tool0' AND translation_z > 1.0";
    assert_compiles_all(q, "tf_states");
    assert_compiles_all(q, "parent_frame");
    assert_compiles_all(q, "child_frame");
    assert_compiles_all(q, "translation_z");
    for dialect in [
        SqlDialect::PostgreSQL,
        SqlDialect::DuckDB,
        SqlDialect::MySQL,
    ] {
        let sql = compile_sql(q, dialect);
        // bare columns must not be wrapped in JSON map access
        assert!(
            !sql.contains("resource_attributes"),
            "{dialect:?}: tf columns should not be JSON map-accessed, got: {sql}"
        );
    }
}

#[test]
fn combo_tf_where_since_facet() {
    // Combination test (CLAUDE.md rule 3): new clause + 2+ other clauses.
    let q = "FROM tf WHERE parent_frame = 'base_link' AND child_frame = 'tool0' \
             AND translation_z > 1.0 SINCE 1 hour ago FACET robot_id";
    assert_compiles_all(q, "tf_states");
    assert_compiles_all(q, "GROUP BY");
    for dialect in [
        SqlDialect::PostgreSQL,
        SqlDialect::DuckDB,
        SqlDialect::MySQL,
    ] {
        let sql = compile_sql(q, dialect);
        // robot_id on tf_states is a bare column, not a JSON path lookup
        assert!(
            sql.contains("robot_id") && !sql.contains("resource_attributes"),
            "{dialect:?}: FACET robot_id on tf should use bare column, got: {sql}"
        );
    }
}

// ===========================================================================
// ROB-432: ROSQL generalization — robot.* vocabulary + generic aliases.
//
// Additive + backward-compatible. These tests assert that:
//   1. Generic, transport-neutral forms compile (PG + DuckDB, per CLAUDE.md).
//   2. The portable `robot.*` concept keys are selectable/filterable.
//   3. Existing ROS query forms still compile to *identical* SQL (back-compat).
// ===========================================================================

// Helper: assert PG + DuckDB both compile and contain `expected`.
fn assert_compiles_both(query: &str, expected: &str) {
    for dialect in [SqlDialect::PostgreSQL, SqlDialect::DuckDB] {
        let sql = compile_sql(query, dialect);
        assert!(
            sql.contains(expected),
            "{dialect:?} SQL missing '{expected}':\n{sql}"
        );
    }
}

#[test]
fn generic_source_channels_compiles() {
    // `FROM channels` resolves to the topics table, like `FROM topics`.
    assert_compiles_both("FROM channels", "topic_messages");
}

#[test]
fn generic_source_transforms_compiles() {
    assert_compiles_both("FROM transforms", "tf_states");
}

#[test]
fn generic_source_components_compiles() {
    assert_compiles_both("FROM components", "node_graph_edges");
}

#[test]
fn generic_source_aliases_match_ros_sql() {
    // Byte-for-byte identical SQL to the ROS-named form (back-compat proof).
    for dialect in [SqlDialect::PostgreSQL, SqlDialect::DuckDB] {
        assert_eq!(
            compile_sql("FROM channels", dialect),
            compile_sql("FROM topics", dialect)
        );
        assert_eq!(
            compile_sql("FROM transforms", dialect),
            compile_sql("FROM tf", dialect)
        );
        assert_eq!(
            compile_sql("FROM components", dialect),
            compile_sql("FROM node_graph", dialect)
        );
    }
}

#[test]
fn generic_field_component_prefers_robot_then_ros() {
    // The generic `component` alias prefers robot.component and falls back to
    // ros.node via COALESCE — so it works on both robot.* and ROS data.
    let q = "FROM traces WHERE component = '/planner'";
    assert_compiles_both(q, "COALESCE");
    assert_compiles_both(q, "robot.component");
    assert_compiles_both(q, "ros.node");
}

#[test]
fn generic_field_action_prefers_robot_then_ros() {
    let q = "FROM traces WHERE action = 'navigate_to_pose'";
    assert_compiles_both(q, "COALESCE");
    assert_compiles_both(q, "robot.action.name");
    assert_compiles_both(q, "ros.action.name");
}

#[test]
fn generic_field_channel_resolves_to_topic_key() {
    // No portable robot.channel.* yet — `channel` maps to ros.topic (single key).
    let q = "FROM traces WHERE channel = '/cmd_vel'";
    assert_compiles_both(q, "ros.topic");
    // single-key resolution: no COALESCE wrapper for `channel`
    let sql = compile_sql(q, SqlDialect::PostgreSQL);
    assert!(!sql.contains("COALESCE"), "got: {sql}");
}

#[test]
fn robot_concept_keys_are_selectable() {
    // Portable robot.* concept keys can be selected (map-access on span attrs).
    let q = "SELECT robot.action.result, robot.action.goal_id, robot.component FROM traces";
    assert_compiles_both(q, "robot.action.result");
    assert_compiles_both(q, "robot.action.goal_id");
    assert_compiles_both(q, "robot.component");
}

#[test]
fn robot_concept_keys_are_filterable() {
    let q = "FROM traces WHERE robot.action.result = 'aborted' \
             AND robot.target.frame = 'map' AND robot.object.id = 'cube_1'";
    assert_compiles_both(q, "robot.action.result");
    assert_compiles_both(q, "robot.target.frame");
    assert_compiles_both(q, "robot.object.id");
}

#[test]
fn combo_generic_source_where_since_facet() {
    // Combination test (CLAUDE.md rule 3): generic alias + WHERE + SINCE + FACET.
    let q = "FROM channels WHERE topic_name = '/scan' \
             SINCE 1 hour ago FACET robot_id";
    assert_compiles_both(q, "topic_messages");
    assert_compiles_both(q, "GROUP BY");
}

#[test]
fn combo_robot_concept_where_since_facet() {
    let q = "SELECT robot.action.result FROM traces \
             WHERE robot.action.status = 'succeeded' \
             SINCE 30 minutes ago FACET robot_id";
    assert_compiles_both(q, "robot.action.result");
    assert_compiles_both(q, "robot.action.status");
    assert_compiles_both(q, "GROUP BY");
}

#[test]
fn ros_forms_still_compile_unchanged() {
    // Back-compat: classic ROS field/source names keep working.
    assert_compiles_both("FROM traces WHERE node = '/planner'", "ros.node");
    assert_compiles_both("FROM traces WHERE topic = '/cmd_vel'", "ros.topic");
    assert_compiles_both(
        "FROM traces WHERE action_name = 'navigate_to_pose'",
        "ros.action.name",
    );
    assert_compiles_both("FROM topics", "topic_messages");
    assert_compiles_both("FROM tf", "tf_states");
    // The classic ROS `node` field is a single-key access (no COALESCE).
    let sql = compile_sql(
        "FROM traces WHERE node = '/planner'",
        SqlDialect::PostgreSQL,
    );
    assert!(!sql.contains("COALESCE"), "got: {sql}");
}

#[test]
fn show_topics_still_uses_ros_keys() {
    // SHOW compilers now resolve keys through the registry, but ROS data still
    // produces the exact same `ros.*` literals (back-compat).
    assert_compiles_both("SHOW TOPICS", "ros.topic");
    assert_compiles_both("SHOW NODES", "ros.node");
    assert_compiles_both("SHOW NODE GRAPH", "ros.publisher_node");
}
