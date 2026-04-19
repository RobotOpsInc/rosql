//! Cross-backend compile parity tests (issue #96, Section 3).
//!
//! For representative queries, compiles to both PostgreSQL and DuckDB and
//! asserts *structural* equivalence — not string equality (dialects legitimately
//! differ) but the same high-level constructs:
//!   - Same table names referenced
//!   - GROUP BY presence matches
//!   - ORDER BY presence matches
//!   - EXISTS / CTE presence matches (for DURING, TRACE, ANOMALY)
//!   - LIMIT presence matches
//!
//! Run with: `cargo test --test compile_parity`

use rosql::drivers::compiler::compile;
use rosql::drivers::dialect::SqlDialect;
use rosql::drivers::otel_registry::default_otel_registry;
use rosql::drivers::BackendCapabilities;

fn caps() -> BackendCapabilities {
    BackendCapabilities {
        topic_data: true,
        recording_index: true,
    }
}

struct ParityAssert {
    pg: String,
    duck: String,
    query: String,
}

impl ParityAssert {
    fn for_query(query: &str) -> Self {
        let ast = rosql::parse(query).unwrap_or_else(|e| panic!("parse failed for {query}: {e:?}"));
        let registry = default_otel_registry();
        let caps = caps();
        let pg = compile(&ast, &registry, &SqlDialect::PostgreSQL, &caps, None)
            .unwrap_or_else(|e| panic!("PG compile failed for {query}: {e}"))
            .sql;
        let duck = compile(&ast, &registry, &SqlDialect::DuckDB, &caps, None)
            .unwrap_or_else(|e| panic!("DuckDB compile failed for {query}: {e}"))
            .sql;
        ParityAssert {
            pg,
            duck,
            query: query.to_string(),
        }
    }

    fn both_contain(&self, substr: &str) -> &Self {
        assert!(
            self.pg.contains(substr),
            "PG missing '{}' for query: {}\nSQL: {}",
            substr,
            self.query,
            self.pg
        );
        assert!(
            self.duck.contains(substr),
            "DuckDB missing '{}' for query: {}\nSQL: {}",
            substr,
            self.query,
            self.duck
        );
        self
    }

    fn both_omit(&self, substr: &str) -> &Self {
        assert!(
            !self.pg.contains(substr),
            "PG unexpectedly contains '{}' for query: {}\nSQL: {}",
            substr,
            self.query,
            self.pg
        );
        assert!(
            !self.duck.contains(substr),
            "DuckDB unexpectedly contains '{}' for query: {}\nSQL: {}",
            substr,
            self.query,
            self.duck
        );
        self
    }

    fn group_by_matches(&self) -> &Self {
        let pg_has = self.pg.contains("GROUP BY");
        let duck_has = self.duck.contains("GROUP BY");
        assert_eq!(
            pg_has, duck_has,
            "GROUP BY presence mismatch for query: {}\nPG has: {pg_has}, DuckDB has: {duck_has}",
            self.query
        );
        self
    }

    fn order_by_matches(&self) -> &Self {
        let pg_has = self.pg.contains("ORDER BY");
        let duck_has = self.duck.contains("ORDER BY");
        assert_eq!(
            pg_has, duck_has,
            "ORDER BY presence mismatch for query: {}\nPG has: {pg_has}, DuckDB has: {duck_has}",
            self.query
        );
        self
    }

    fn limit_matches(&self) -> &Self {
        let pg_has = self.pg.contains("LIMIT");
        let duck_has = self.duck.contains("LIMIT");
        assert_eq!(
            pg_has, duck_has,
            "LIMIT presence mismatch for query: {}\nPG has: {pg_has}, DuckDB has: {duck_has}",
            self.query
        );
        self
    }

    fn cte_matches(&self, cte_name: &str) -> &Self {
        let pg_has = self.pg.contains(cte_name);
        let duck_has = self.duck.contains(cte_name);
        assert_eq!(
            pg_has, duck_has,
            "CTE '{}' presence mismatch for query: {}\nPG has: {pg_has}, DuckDB has: {duck_has}",
            cte_name, self.query
        );
        self
    }
}

#[test]
fn parity_basic_where_since() {
    ParityAssert::for_query("FROM traces WHERE status = 'ERROR' SINCE 1 hour ago")
        .both_contain("otel_traces")
        .both_contain("status_code")
        .both_omit("GROUP BY")
        .order_by_matches()
        .limit_matches();
}

#[test]
fn parity_count_facet() {
    ParityAssert::for_query("SELECT COUNT(*) FROM traces WHERE status = 'ERROR' FACET robot_id")
        .both_contain("otel_traces")
        .both_contain("robot.id")
        .both_contain("COUNT")
        .group_by_matches();
}

#[test]
fn parity_avg_timeseries() {
    ParityAssert::for_query("SELECT AVG(duration) FROM traces TIMESERIES 5 min SINCE 6 hours ago")
        .both_contain("otel_traces")
        .both_contain("time_bucket")
        .both_contain("AVG")
        .group_by_matches()
        .order_by_matches();
}

#[test]
fn parity_enrich_with() {
    // ENRICH WITH: primary SQL should be identical across dialects except for
    // interval syntax. Assert both reference the same table and same WHERE key.
    ParityAssert::for_query("FROM traces WHERE status = 'ERROR' ENRICH WITH logs SINCE 1 hour ago")
        .both_contain("otel_traces")
        .both_contain("status_code")
        .both_omit("GROUP BY");
}

#[test]
fn parity_for_robot_scope() {
    ParityAssert::for_query("FOR ROBOT 'r1' FROM traces WHERE duration > 500 ms")
        .both_contain("otel_traces")
        .both_contain("robot.id")
        .both_contain("r1")
        .both_omit("GROUP BY");
}

#[test]
fn parity_topic_rate() {
    ParityAssert::for_query("SELECT TOPIC_RATE('/cmd_vel') FROM metrics")
        .both_contain("otel_metrics")
        .both_contain("/cmd_vel")
        .both_contain("AVG");
}

#[test]
fn parity_action_success_rate() {
    ParityAssert::for_query("SELECT ACTION_SUCCESS_RATE('/navigate_to_pose') FROM traces")
        .both_contain("otel_traces")
        .both_contain("navigate_to_pose")
        .both_contain("succeeded")
        .both_contain("CASE WHEN");
}

#[test]
fn parity_approx_percentile() {
    ParityAssert::for_query("SELECT APPROX_PERCENTILE(duration, 95) FROM traces")
        .both_contain("otel_traces")
        .both_contain("0.95");
}

#[test]
fn parity_trace_cte() {
    ParityAssert::for_query("TRACE 'abc123'")
        .both_contain("WITH RECURSIVE")
        .both_contain("trace_tree")
        .both_contain("abc123")
        .cte_matches("trace_tree");
}

#[test]
fn parity_path_deviation() {
    ParityAssert::for_query("PATH DEVIATION FOR ROBOT 'robot_sim_001' SINCE 1 hour ago")
        .both_contain("planned_path")
        .both_contain("actual_poses")
        .both_contain("lateral_deviation_m")
        .cte_matches("planned_path")
        .cte_matches("actual_poses")
        .order_by_matches();
}

#[test]
fn parity_anomaly() {
    ParityAssert::for_query(
        "ANOMALY(duration) COMPARED TO last week FACET robot_id SINCE 7 days ago",
    )
    .both_contain("current_stats")
    .both_contain("baseline_stats")
    .both_contain("z_score")
    .both_contain("is_anomalous")
    .cte_matches("current_stats")
    .cte_matches("baseline_stats")
    .group_by_matches();
}

#[test]
fn parity_show_span_summary() {
    ParityAssert::for_query("SHOW SPAN SUMMARY FOR ROBOT 'r1' SINCE 1 hour ago")
        .both_contain("span_name")
        .both_contain("robot.id")
        .both_contain("AVG")
        .group_by_matches();
}

#[test]
fn parity_show_deployments() {
    ParityAssert::for_query("SHOW DEPLOYMENTS SINCE 7 days ago")
        .both_contain("service.version")
        .both_contain("deployment.environment")
        .group_by_matches();
}

#[test]
fn parity_within_gps() {
    ParityAssert::for_query(
        "FROM odom WHERE position WITHIN 500 m OF (37.7749, -122.4194) SINCE 1 hour ago",
    )
    .both_contain("topic_messages")
    .both_contain("ASIN")
    .both_contain("6371000")
    .both_omit("GROUP BY");
}

#[test]
fn parity_pipeline_facet() {
    ParityAssert::for_query(
        "FROM traces | WHERE status = 'ERROR' | FACET robot_id | SINCE 1 hour ago",
    )
    .both_contain("otel_traces")
    .both_contain("robot.id")
    .group_by_matches();
}
