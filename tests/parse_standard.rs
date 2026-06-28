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
fn snapshot_from_node_graph() {
    let ast = parse("FROM node_graph").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_from_joints() {
    let ast = parse("FROM joints").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// ROB-432: generic, transport-neutral data-source aliases resolve to the same
// AST as their ROS-named forms (`channels` ↔ topics, `transforms` ↔ tf,
// `components` ↔ node_graph).
#[test]
fn snapshot_from_channels_alias() {
    let ast = parse("FROM channels").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_from_transforms_alias() {
    let ast = parse("FROM transforms").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_from_components_alias() {
    let ast = parse("FROM components").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// The generic aliases parse to the *same* AST as the ROS-named forms.
#[test]
fn generic_source_aliases_match_ros_forms() {
    assert_eq!(
        parse("FROM channels").unwrap(),
        parse("FROM topics").unwrap()
    );
    assert_eq!(parse("FROM transforms").unwrap(), parse("FROM tf").unwrap());
    assert_eq!(
        parse("FROM components").unwrap(),
        parse("FROM node_graph").unwrap()
    );
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

// ── TIMESERIES ───────────────────────────────────────────────────────────────

#[test]
fn snapshot_timeseries_basic() {
    let ast = parse(
        "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' TIMESERIES 5 min SINCE 6 hours ago",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_timeseries_with_facet() {
    let ast = parse(
        "SELECT AVG(duration) FROM traces TIMESERIES 1 min FACET action_name SINCE 1 hour ago",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_timeseries_1_hour() {
    let ast = parse("FROM traces TIMESERIES 1 hour SINCE 24 hours ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// ── ENRICH WITH ──────────────────────────────────────────────────────────────

#[test]
fn snapshot_enrich_with_logs() {
    let ast =
        parse("FROM traces WHERE status = 'ERROR' ENRICH WITH logs SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_enrich_with_limit() {
    let ast =
        parse("FROM traces WHERE status = 'ERROR' ENRICH WITH logs LIMIT 200 SINCE 1 hour ago")
            .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_enrich_with_sample_full() {
    let ast = parse("FROM traces ENRICH WITH joint_states SAMPLE FULL SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_enrich_multiple() {
    let ast = parse("SELECT * FROM traces WHERE status = 'ERROR' ENRICH WITH logs ENRICH WITH recordings SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// ── Geospatial WITHIN ────────────────────────────────────────────────────────

#[test]
fn snapshot_within_gps() {
    let ast =
        parse("FROM odom WHERE position WITHIN 500 m OF (37.7749, -122.4194) SINCE 1 hour ago")
            .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_within_local() {
    let ast = parse("FROM odom WHERE position WITHIN 2 m OF POSITION (1.5, 3.0) SINCE 1 hour ago")
        .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// ── Array-indexed field access ───────────────────────────────────────────────

#[test]
fn snapshot_field_access_array_index() {
    let ast = parse(
        "FROM joint_states WHERE fields['position[0]'] > 1.5 FOR ROBOT 'arm_01' SINCE 1 hour ago",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}
