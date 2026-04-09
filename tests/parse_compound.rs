//! Snapshot tests for compound clause queries.

use rosql::parse;

// ── Removed syntax — deprecation errors ─────────────────────────────────────

#[test]
fn message_journey_removed() {
    let errs = parse("MESSAGE JOURNEY FOR TRACE 'abc123def456'").unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("MESSAGE JOURNEY is removed")),
        "got: {errs:?}"
    );
}

#[test]
fn message_paths_removed() {
    let errs = parse("MESSAGE PATHS FOR TOPIC '/cmd_vel' SINCE 1 hour ago").unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("MESSAGE PATHS is removed")),
        "got: {errs:?}"
    );
}

#[test]
fn message_path_removed() {
    let errs =
        parse("MESSAGE PATH FROM TOPIC '/scan' TO NODE '/local_costmap_node' SINCE 1 day ago")
            .unwrap_err();
    assert!(
        errs.iter()
            .any(|e| e.to_string().contains("MESSAGE PATH is removed")),
        "got: {errs:?}"
    );
}

// ── Active compound clauses ──────────────────────────────────────────────────

#[test]
fn snapshot_trace() {
    let ast = parse("TRACE 'abc123def456'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_message_flow() {
    let ast = parse("MESSAGE FLOW FROM TOPIC '/cmd_vel' SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_message_flow_to_node() {
    let ast =
        parse("MESSAGE FLOW FROM TOPIC '/scan' TO NODE '/local_costmap_node' SINCE 1 day ago")
            .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_message_flow_to_topic() {
    let ast = parse("MESSAGE FLOW FROM TOPIC '/scan' TO TOPIC '/costmap' SINCE 1 day ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_health() {
    let ast = parse("HEALTH() FOR ROBOT 'robot_42' SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_health_faceted() {
    let ast = parse("HEALTH() SINCE 30 minutes ago FACET robot_id").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_anomaly() {
    let ast =
        parse("ANOMALY(duration) COMPARED TO fleet SINCE 24 hours ago FACET robot_id").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_path_deviation() {
    let ast = parse("PATH DEVIATION FOR ROBOT 'robot_42' SINCE yesterday").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_correlate() {
    let ast = parse("CORRELATE WITH metrics SINCE 7 days ago FACET robot_id").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_recording() {
    let ast = parse("SHOW RECORDING SINCE yesterday").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_deployments() {
    let ast = parse("SHOW DEPLOYMENTS FOR ROBOT 'robot_42' SINCE 30 days ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_span_summary() {
    let ast = parse("SHOW SPAN SUMMARY SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_plans() {
    let ast = parse("SHOW PLANS FOR TRACE 'abc123'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_plans_scoped() {
    let ast = parse("SHOW PLANS FOR ROBOT 'robot_42' SINCE 1 day ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// ── SHOW TOPICS / SHOW NODES / SHOW NODE GRAPH ──────────────────────────────

#[test]
fn snapshot_show_topics() {
    let ast = parse("SHOW TOPICS FOR ROBOT 'robot_42' SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_topics_where() {
    let ast = parse("SHOW TOPICS WHERE topic_name LIKE '/camera%' SINCE 30 minutes ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_nodes() {
    let ast = parse("SHOW NODES FOR ROBOT 'robot_42' SINCE 30 minutes ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_node_graph() {
    let ast = parse("SHOW NODE GRAPH FOR ROBOT 'robot_42' SINCE 30 minutes ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_show_topics_no_scope() {
    let ast = parse("SHOW TOPICS SINCE 6 hours ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn show_node_without_graph_errors() {
    let err = parse("SHOW NODE FOR ROBOT 'robot_42'").unwrap_err();
    assert!(
        err.iter()
            .any(|e| e.to_string().contains("expected GRAPH after SHOW NODE")),
        "got: {err:?}"
    );
}

// ── PATH DEVIATION (redesigned v0.4.3) ──────────────────────────────────────

#[test]
fn snapshot_path_deviation_for_trace() {
    let ast = parse("PATH DEVIATION FOR TRACE 'abc123'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_path_deviation_for_robot_since() {
    let ast = parse("PATH DEVIATION FOR ROBOT 'r1' SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_path_deviation_plan_index() {
    let ast = parse("PATH DEVIATION PLAN 0 FOR TRACE 'abc'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_path_deviation_plan_negative() {
    let ast = parse("PATH DEVIATION PLAN -1 FOR TRACE 'abc'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// ── JOINT DEVIATION ─────────────────────────────────────────────────────────

#[test]
fn snapshot_joint_deviation_for_trace() {
    let ast = parse("JOINT DEVIATION FOR TRACE 'abc'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_joint_deviation_for_robot() {
    let ast = parse("JOINT DEVIATION FOR ROBOT 'arm_01' SINCE yesterday").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// ── ANOMALY (redesigned v0.4.3) ──────────────────────────────────────────────

#[test]
fn snapshot_anomaly_from_traces() {
    let ast = parse(
        "ANOMALY(duration) FROM traces COMPARED TO last week FACET robot_id SINCE 7 days ago",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_anomaly_last_24h() {
    let ast = parse("ANOMALY(duration) COMPARED TO last 24 hours SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn anomaly_missing_compared_to_errors() {
    let errs = parse("ANOMALY(duration) SINCE 1 hour ago").unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("COMPARED TO")),
        "expected COMPARED TO error, got: {errs:?}"
    );
}

// ── SHOW JOINTS ──────────────────────────────────────────────────────────────

#[test]
fn snapshot_show_joints() {
    let ast = parse("SHOW JOINTS FOR ROBOT 'arm_01'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}
