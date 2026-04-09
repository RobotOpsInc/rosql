//! Snapshot tests for compound clause queries.

use rosql::parse;

// ── Removed syntax — deprecation errors ─────────────────────────────────────

#[test]
fn message_journey_removed() {
    let errs = parse("MESSAGE JOURNEY FOR TRACE 'abc123def456'").unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("MESSAGE JOURNEY is removed")),
        "got: {errs:?}"
    );
}

#[test]
fn message_paths_removed() {
    let errs = parse("MESSAGE PATHS FOR TOPIC '/cmd_vel' SINCE 1 hour ago").unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("MESSAGE PATHS is removed")),
        "got: {errs:?}"
    );
}

#[test]
fn message_path_removed() {
    let errs =
        parse("MESSAGE PATH FROM TOPIC '/scan' TO NODE '/local_costmap_node' SINCE 1 day ago")
            .unwrap_err();
    assert!(
        errs.iter().any(|e| e.to_string().contains("MESSAGE PATH is removed")),
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
    let ast =
        parse("MESSAGE FLOW FROM TOPIC '/scan' TO TOPIC '/costmap' SINCE 1 day ago").unwrap();
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
