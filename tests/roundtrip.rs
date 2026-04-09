//! Roundtrip tests: parse → serialize to JSON → deserialize → assert equality.

use rosql::parse;

fn roundtrip(source: &str) {
    let ast = parse(source).unwrap_or_else(|e| panic!("parse failed for '{source}': {e:?}"));
    let json = serde_json::to_string(&ast).expect("serialize failed");
    let roundtripped: rosql::Query = serde_json::from_str(&json).expect("deserialize failed");
    assert_eq!(ast, roundtripped, "roundtrip failed for '{source}'");
}

#[test]
fn roundtrip_basic_select() {
    roundtrip("SELECT * FROM logs");
}

#[test]
fn roundtrip_full_query() {
    roundtrip(
        "SELECT span_name, duration FROM logs \
         WHERE duration > 500 ms \
         FACET robot_id \
         SINCE 30 minutes ago \
         ORDER BY duration DESC \
         LIMIT 10",
    );
}

#[test]
fn roundtrip_pipeline() {
    roundtrip("FROM logs | WHERE duration > 500 ms | FACET robot_id | COMPARE TO last week");
}

#[test]
fn roundtrip_compound_health() {
    roundtrip("HEALTH() FOR ROBOT 'robot_42' SINCE 1 hour ago");
}

#[test]
fn roundtrip_trace() {
    roundtrip("TRACE 'abc123'");
}

#[test]
fn roundtrip_message_flow() {
    roundtrip("MESSAGE FLOW FROM TOPIC '/cmd_vel' SINCE 1 hour ago");
}

#[test]
fn roundtrip_message_flow_to_node() {
    roundtrip("MESSAGE FLOW FROM TOPIC '/scan' TO NODE '/local_costmap_node'");
}

#[test]
fn roundtrip_show_deployments() {
    roundtrip("SHOW DEPLOYMENTS FOR ROBOT 'r1' SINCE 30 days ago");
}

#[test]
fn roundtrip_show_plans() {
    roundtrip("SHOW PLANS FOR TRACE 'abc123'");
}

#[test]
fn roundtrip_compare_to_version() {
    roundtrip("FROM traces COMPARE TO VERSION 'v1.2.3'");
}

#[test]
fn roundtrip_compare_version_pair() {
    roundtrip("FROM traces COMPARE VERSION 'v1.0' TO VERSION 'v2.0'");
}

#[test]
fn roundtrip_scoped_multi() {
    roundtrip("SELECT * FROM logs FOR ROBOT 'r1' FOR VERSION 'v1.0' FOR ENVIRONMENT 'prod'");
}

#[test]
fn roundtrip_compound_anomaly() {
    roundtrip("ANOMALY(duration) COMPARED TO fleet SINCE 24 hours ago FACET robot_id");
}

#[test]
fn roundtrip_lifecycle_anchor() {
    roundtrip("FROM logs SINCE last deployment");
}

#[test]
fn roundtrip_topic_alias() {
    roundtrip("FROM odom SINCE 10 minutes ago");
}

#[test]
fn roundtrip_for_fleet() {
    roundtrip("FROM metrics SINCE 1 hour ago FOR FLEET");
}

#[test]
fn roundtrip_path_deviation() {
    roundtrip("PATH DEVIATION FOR ROBOT 'robot_42' SINCE yesterday");
}

#[test]
fn roundtrip_path_deviation_trace() {
    roundtrip("PATH DEVIATION FOR TRACE 'abc123'");
}

#[test]
fn roundtrip_path_deviation_plan_index() {
    roundtrip("PATH DEVIATION PLAN 0 FOR TRACE 'abc'");
}

#[test]
fn roundtrip_joint_deviation() {
    roundtrip("JOINT DEVIATION FOR ROBOT 'arm_01' SINCE 1 hour ago");
}

#[test]
fn roundtrip_anomaly_redesigned() {
    roundtrip("ANOMALY(duration) COMPARED TO last week FACET robot_id SINCE 7 days ago");
}

#[test]
fn roundtrip_anomaly_last_24h() {
    roundtrip("ANOMALY(duration) COMPARED TO last 24 hours SINCE 1 hour ago");
}

#[test]
fn roundtrip_show_joints() {
    roundtrip("SHOW JOINTS FOR ROBOT 'arm_01'");
}

#[test]
fn roundtrip_within_gps() {
    roundtrip("FROM odom WHERE position WITHIN 500 m OF (37.7749, -122.4194) SINCE 1 hour ago");
}

#[test]
fn roundtrip_within_local() {
    roundtrip("FROM odom WHERE position WITHIN 2 m OF POSITION (1.5, 3.0) SINCE 1 hour ago");
}
