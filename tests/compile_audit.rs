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
