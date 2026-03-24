//! Snapshot tests for compound clause queries.

use rosql::parse;

#[test]
fn snapshot_message_journey() {
    let ast = parse("MESSAGE JOURNEY FOR TRACE 'abc123def456'").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_message_paths() {
    let ast = parse("MESSAGE PATHS FOR TOPIC '/cmd_vel' SINCE 1 hour ago").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_message_path_from_to() {
    let ast =
        parse("MESSAGE PATH FROM TOPIC '/scan' TO NODE '/local_costmap_node' SINCE 1 day ago")
            .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_trace() {
    let ast = parse("TRACE 'abc123def456'").unwrap();
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
