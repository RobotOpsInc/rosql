//! Snapshot tests for standard ROSQL queries.

use rosql::parse;

#[test]
fn snapshot_basic_select() {
    let ast = parse("SELECT span_name, duration FROM logs").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_select_star() {
    let ast = parse("SELECT * FROM traces").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_from_shorthand() {
    let ast = parse("FROM metrics").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_full_query() {
    let ast = parse(
        "SELECT span_name, AVG(duration) AS avg_dur \
         FROM logs \
         WHERE duration > 500 ms AND ros.node = '/planner' \
         FACET robot_id \
         SINCE 30 minutes ago \
         USING ROS_TIME \
         ORDER BY duration DESC \
         LIMIT 10 \
         COMPARE TO last week",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_for_robot() {
    let ast = parse("FOR ROBOT 'robot_42' FROM logs SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_topic_alias() {
    let ast = parse("FROM odom SINCE 10 minutes ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_between_timestamps() {
    let ast = parse("FROM logs BETWEEN '2026-03-18T14:00:00Z' AND '2026-03-18T15:00:00Z'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_unix_epoch() {
    let ast = parse("FROM logs SINCE 1742306400").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_lifecycle_anchor() {
    let ast = parse("FROM logs SINCE last action failure").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_where_in() {
    let ast = parse("FROM logs WHERE severity IN ('ERROR', 'WARN')").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_bracket_access() {
    let ast = parse("FROM logs WHERE fields['my_value'] > 42").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_aggregation_with_args() {
    let ast = parse("SELECT PERCENTILE(duration, 95) FROM logs").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_limit_offset() {
    let ast = parse("FROM logs LIMIT 20 OFFSET 40").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_offset_only() {
    let ast = parse("FROM logs OFFSET 10").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_composable_scope() {
    let ast = parse(
        "SELECT * FROM logs \
         FOR ROBOT 'robot_sim_001' FOR VERSION 'v2.3.1' FOR ENVIRONMENT 'production' \
         SINCE 1 hour ago",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_for_session() {
    let ast = parse("FROM traces FOR SESSION 'sess_abc123' SINCE 30 minutes ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}
