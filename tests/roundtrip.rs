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
fn roundtrip_compound_message_journey() {
    roundtrip("MESSAGE JOURNEY FOR TRACE 'abc123'");
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
