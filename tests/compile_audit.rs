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

// ── Implemented aggregation functions ───────────────────────────────────────

#[test]
fn topic_rate_compiles_to_subquery() {
    let sql = compile_sql("SELECT TOPIC_RATE() FROM metrics", SqlDialect::PostgreSQL);
    assert!(sql.contains("otel_metrics"), "got: {sql}");
    assert!(sql.contains("ros2.topic.message_rate"), "got: {sql}");
    assert!(sql.contains("AVG"), "got: {sql}");
    // Uses the registry-resolved column name, not the ROSQL alias.
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
    let sql = compile_sql(
        "SELECT TOPIC_RATE('/cmd_vel') FROM metrics",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("otel_metrics"), "got: {sql}");
    // Topic filter uses JSON extraction from the attributes column, not a bare column.
    assert!(sql.contains("attributes"), "got: {sql}");
    assert!(sql.contains("topic"), "got: {sql}");
    assert!(
        !sql.contains("topic_name"),
        "should not use bare topic_name column, got: {sql}"
    );
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
    // Uses JSON extraction, not a bare column name.
    assert!(
        sql.contains("span_attributes"),
        "expected span_attributes JSON access, got: {sql}"
    );
    assert!(sql.contains("ros.action.status"), "got: {sql}");
}

#[test]
fn action_success_rate_with_arg() {
    let sql = compile_sql(
        "SELECT ACTION_SUCCESS_RATE('/navigate_to_pose') FROM traces",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("navigate_to_pose"), "got: {sql}");
    assert!(sql.contains("succeeded"), "got: {sql}");
    // action_name filter also uses JSON extraction.
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
    // Explicit aggregation + FACET must include the facet column in SELECT.
    let sql = compile_sql(
        "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' FACET service_name",
        SqlDialect::DuckDB,
    );
    // service_name is a real column on otel_traces — should appear in SELECT and GROUP BY.
    let col = r#""service_name""#;
    assert!(
        sql.contains(&format!("SELECT {col}")) || sql.contains(&format!("{col}, COUNT")),
        "facet column missing from SELECT, got: {sql}"
    );
    assert!(sql.contains("GROUP BY"), "got: {sql}");
}

#[test]
fn facet_robot_id_resolves_json_on_traces() {
    // robot_id on otel_traces must resolve to resource_attributes->>'robot.id', not a bare column.
    let sql = compile_sql(
        "SELECT COUNT(*) FROM traces FACET robot_id",
        SqlDialect::DuckDB,
    );
    assert!(
        sql.contains("resource_attributes") && sql.contains("robot.id"),
        "expected JSON extraction for robot_id on otel_traces, got: {sql}"
    );
}

#[test]
fn facet_robot_id_bare_on_topic_messages() {
    // robot_id on topic_messages must resolve to the bare column, not JSON extraction.
    let sql = compile_sql("FROM odom FACET robot_id", SqlDialect::DuckDB);
    assert!(
        sql.contains(r#""robot_id""#),
        "expected bare robot_id column on topic_messages, got: {sql}"
    );
    assert!(
        !sql.contains("resource_attributes"),
        "should not use resource_attributes on topic_messages, got: {sql}"
    );
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
fn anomaly_compiles_two_cte() {
    // ANOMALY now compiles to a two-phase CTE with z-score output.
    let sql = compile_sql(
        "ANOMALY(duration) COMPARED TO last week FACET robot_id SINCE 7 days ago",
        SqlDialect::DuckDB,
    );
    assert!(sql.contains("current_stats"), "got: {sql}");
    assert!(sql.contains("baseline_stats"), "got: {sql}");
    assert!(sql.contains("z_score"), "got: {sql}");
    assert!(sql.contains("is_anomalous"), "got: {sql}");
    assert!(sql.contains("direction"), "got: {sql}");
}

#[test]
fn anomaly_last_24h_baseline() {
    let sql = compile_sql(
        "ANOMALY(duration) COMPARED TO last 24 hours FACET robot_id SINCE 12 hours ago",
        SqlDialect::PostgreSQL,
    );
    // Should reference a 48-hour and 24-hour window for the baseline
    assert!(sql.contains("48"), "expected 48-hour window: {sql}");
    assert!(sql.contains("24"), "expected 24-hour window: {sql}");
    assert!(sql.contains("baseline_stats"), "got: {sql}");
}

#[test]
fn anomaly_missing_compared_to_is_parse_error() {
    // ANOMALY without COMPARED TO is now a parse error (not NotImplemented).
    let result = rosql::parse("ANOMALY(duration)");
    assert!(result.is_err(), "expected parse error");
}

#[test]
fn path_deviation_compiles() {
    // PATH DEVIATION now compiles to a two-CTE SQL query.
    let sql = compile_sql(
        "PATH DEVIATION FOR ROBOT 'r1' SINCE yesterday",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("planned_path"), "got: {sql}");
    assert!(sql.contains("actual_poses"), "got: {sql}");
    assert!(sql.contains("lateral_deviation_m"), "got: {sql}");
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
    let sql = compile_sql(
        "JOINT DEVIATION FOR ROBOT 'arm_01' SINCE 2 hours ago",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("planned_joints"), "got: {sql}");
    assert!(sql.contains("actual_joints"), "got: {sql}");
    assert!(sql.contains("/joint_trajectory"), "got: {sql}");
    assert!(sql.contains("/joint_states"), "got: {sql}");
    assert!(sql.contains("joint_error_rad"), "got: {sql}");
}

#[test]
fn show_joints_compiles() {
    let sql = compile_sql("SHOW JOINTS FOR ROBOT 'arm_01'", SqlDialect::PostgreSQL);
    assert!(sql.contains("robot_joint_map"), "got: {sql}");
    assert!(sql.contains("arm_01"), "got: {sql}");
}

#[test]
fn within_local_compiles() {
    let sql = compile_sql(
        "FROM odom WHERE position WITHIN 2 m OF POSITION (1.5, 3.0) SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("SQRT"), "expected Euclidean formula: {sql}");
    assert!(sql.contains("POWER"), "expected POWER: {sql}");
    assert!(sql.contains("2"), "expected radius 2: {sql}");
}

#[test]
fn within_gps_compiles() {
    let sql = compile_sql(
        "FROM odom WHERE position WITHIN 500 m OF (37.7749, -122.4194) SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("ASIN"), "expected Haversine ASIN: {sql}");
    assert!(sql.contains("6371000"), "expected Earth radius: {sql}");
    assert!(sql.contains("500"), "expected 500 m radius: {sql}");
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

// ── P0: Scope compilation ────────────────────────────────────────────────────

#[test]
fn for_robot_compiles_to_where_clause() {
    let sql = compile_sql("SELECT * FROM logs FOR ROBOT 'r1'", SqlDialect::PostgreSQL);
    assert!(sql.contains("robot.id"), "got: {sql}");
    assert!(sql.contains("r1"), "got: {sql}");
}

#[test]
fn for_version_compiles_to_where_clause() {
    let sql = compile_sql(
        "SELECT * FROM logs FOR VERSION 'v1.2.3'",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("service.version"), "got: {sql}");
    assert!(sql.contains("v1.2.3"), "got: {sql}");
}

#[test]
fn for_environment_compiles_to_where_clause() {
    let sql = compile_sql(
        "SELECT * FROM logs FOR ENVIRONMENT 'production'",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("deployment.environment"), "got: {sql}");
    assert!(sql.contains("production"), "got: {sql}");
}

#[test]
fn for_session_compiles_to_where_clause() {
    let sql = compile_sql(
        "SELECT * FROM logs FOR SESSION 'sess_abc'",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("ros.session.id"), "got: {sql}");
    assert!(sql.contains("sess_abc"), "got: {sql}");
}

#[test]
fn composable_scope_emits_all_filters() {
    let sql = compile_sql(
        "SELECT * FROM logs FOR ROBOT 'r1' FOR VERSION 'v1.0' FOR ENVIRONMENT 'prod'",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("robot.id"), "got: {sql}");
    assert!(sql.contains("service.version"), "got: {sql}");
    assert!(sql.contains("deployment.environment"), "got: {sql}");
}

#[test]
fn trace_with_scope_compiles_to_cte_with_filter() {
    let sql = compile_sql("TRACE 'abc123' FOR ROBOT 'r1'", SqlDialect::PostgreSQL);
    assert!(sql.contains("WITH RECURSIVE trace_tree"), "got: {sql}");
    assert!(sql.contains("robot.id"), "got: {sql}");
}

// ── SHOW commands ─────────────────────────────────────────────────────────────

#[test]
fn show_deployments_compiles() {
    let sql = compile_sql("SHOW DEPLOYMENTS SINCE 30 days ago", SqlDialect::PostgreSQL);
    assert!(sql.contains("service.version"), "got: {sql}");
    assert!(sql.contains("deployment.environment"), "got: {sql}");
    assert!(sql.contains("GROUP BY"), "got: {sql}");
}

#[test]
fn show_span_summary_compiles() {
    let sql = compile_sql("SHOW SPAN SUMMARY SINCE 1 hour ago", SqlDialect::PostgreSQL);
    assert!(sql.contains("span_name"), "got: {sql}");
    assert!(sql.contains("AVG"), "got: {sql}");
    assert!(sql.contains("GROUP BY"), "got: {sql}");
}

#[test]
fn show_plans_compiles() {
    let sql = compile_sql("SHOW PLANS FOR TRACE 'abc123'", SqlDialect::PostgreSQL);
    assert!(sql.contains("ros.plan.id"), "got: {sql}");
    assert!(sql.contains("abc123"), "got: {sql}");
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
fn show_topics_compiles_postgres() {
    let sql = compile_sql(
        "SHOW TOPICS FOR ROBOT 'robot_42' SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("ros.topic"), "got: {sql}");
    assert!(sql.contains("topic_name"), "got: {sql}");
    assert!(sql.contains("avg_rate_hz"), "got: {sql}");
    assert!(sql.contains("publishers"), "got: {sql}");
    assert!(sql.contains("subscribers"), "got: {sql}");
    assert!(sql.contains("robot.id"), "got: {sql}");
}

#[test]
fn show_topics_compiles_duckdb() {
    let sql = compile_sql("SHOW TOPICS SINCE 6 hours ago", SqlDialect::DuckDB);
    assert!(sql.contains("ros.topic"), "got: {sql}");
    assert!(sql.contains("GROUP BY"), "got: {sql}");
}

#[test]
fn show_nodes_compiles_postgres() {
    let sql = compile_sql(
        "SHOW NODES FOR ROBOT 'robot_42' SINCE 30 minutes ago",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("ros.node"), "got: {sql}");
    assert!(sql.contains("node_name"), "got: {sql}");
    assert!(sql.contains("topics_published"), "got: {sql}");
    assert!(sql.contains("error_count"), "got: {sql}");
    assert!(sql.contains("last_seen"), "got: {sql}");
}

#[test]
fn show_node_graph_compiles_postgres() {
    let sql = compile_sql(
        "SHOW NODE GRAPH FOR ROBOT 'robot_42' SINCE 30 minutes ago",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("source_node"), "got: {sql}");
    assert!(sql.contains("target_node"), "got: {sql}");
    assert!(sql.contains("DISTINCT"), "got: {sql}");
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
fn during_compiles_to_exists_subquery_postgres() {
    let sql = compile_sql(
        "DURING(FROM topics WHERE topic_name = '/battery_state' AND fields['percentage'] < 15) \
         SINCE 6 hours ago",
        SqlDialect::PostgreSQL,
    );
    // DURING should produce a EXISTS subquery against the inner data source
    assert!(
        sql.contains("topic_messages"),
        "expected topic_messages in DURING subquery, got: {sql}"
    );
    assert!(
        sql.contains("otel_traces"),
        "expected outer otel_traces, got: {sql}"
    );
    assert!(
        sql.contains("battery_state"),
        "expected topic filter, got: {sql}"
    );
}

#[test]
fn during_compiles_to_exists_subquery_duckdb() {
    let sql = compile_sql(
        "DURING(FROM metrics WHERE metric_name = 'system.cpu.utilization')",
        SqlDialect::DuckDB,
    );
    assert!(
        sql.contains("otel_metrics"),
        "expected otel_metrics in DURING subquery, got: {sql}"
    );
    assert!(
        sql.contains("otel_traces"),
        "expected outer otel_traces, got: {sql}"
    );
}

// ── TIMESERIES ───────────────────────────────────────────────────────────────

#[test]
fn timeseries_compiles_duckdb() {
    let sql = compile_sql(
        "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' TIMESERIES 5 min SINCE 6 hours ago",
        SqlDialect::DuckDB,
    );
    assert!(sql.contains("time_bucket"), "got: {sql}");
    assert!(sql.contains("GROUP BY"), "got: {sql}");
    assert!(sql.contains("ORDER BY time_bucket ASC"), "got: {sql}");
    assert!(sql.contains("INTERVAL '5 minutes'"), "got: {sql}");
}

#[test]
fn timeseries_compiles_postgres() {
    let sql = compile_sql(
        "SELECT COUNT(*) FROM traces TIMESERIES 1 hour SINCE 24 hours ago",
        SqlDialect::PostgreSQL,
    );
    assert!(sql.contains("time_bucket"), "got: {sql}");
    assert!(sql.contains("date_bin"), "got: {sql}");
    assert!(sql.contains("GROUP BY"), "got: {sql}");
}

#[test]
fn timeseries_bare_field_wrapped_in_avg_duckdb() {
    // A bare field selection (not an aggregation) in a TIMESERIES+FACET query must be
    // auto-wrapped in AVG() so the generated SQL is valid under GROUP BY.
    // Regression: previously emitted bare "value" causing DuckDB binder error.
    let sql = compile_sql(
        "SELECT cpu_usage FROM metrics TIMESERIES 2 min FACET robot_id SINCE 45 min ago",
        SqlDialect::DuckDB,
    );
    assert!(sql.contains("AVG("), "bare field not wrapped in AVG: {sql}");
    assert!(sql.contains("GROUP BY"), "got: {sql}");
    assert!(sql.contains("time_bucket"), "got: {sql}");
}

#[test]
fn timeseries_composes_with_facet_duckdb() {
    let sql = compile_sql(
        "SELECT AVG(duration) FROM traces TIMESERIES 1 min FACET action_name SINCE 1 hour ago",
        SqlDialect::DuckDB,
    );
    assert!(sql.contains("time_bucket"), "got: {sql}");
    // action_name is resolved to span_attributes json access in OTel registry
    assert!(
        sql.contains("action_name") || sql.contains("action.name"),
        "got: {sql}"
    );
    // Both time_bucket and facet dimension should appear in GROUP BY
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
    // Primary SQL should be the same as without enrichment
    let enriched = compile_sql(
        "FROM traces WHERE status = 'ERROR' ENRICH WITH logs SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    let plain = compile_sql(
        "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago",
        SqlDialect::PostgreSQL,
    );
    assert_eq!(enriched, plain, "ENRICH WITH should not modify primary SQL");
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
    assert_eq!(viz.y_axis.as_deref(), Some("duration"));
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

    // Query 3: Fleet CPU over time — TIMESERIES + FACET → StackedLineChart
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

    // Query 5: Slowest spans → HorizontalBars
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
