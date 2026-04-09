//! Snapshot tests for pipeline ROSQL queries.

use rosql::parse;

#[test]
fn snapshot_basic_pipeline() {
    let ast = parse("FROM logs | WHERE duration > 500 ms | FACET robot_id").unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_pipeline_with_compare() {
    let ast = parse(
        "FROM logs \
         | WHERE duration > 500 ms \
         | FACET robot_id \
         | COMPARE TO last week",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_pipeline_with_limit() {
    let ast = parse(
        "FROM logs \
         | WHERE ros.node = '/planner' \
         | ORDER BY duration DESC \
         | LIMIT 20",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_pipeline_with_offset() {
    let ast = parse(
        "FROM logs \
         | WHERE ros.node = '/planner' \
         | LIMIT 10 \
         | OFFSET 20",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}
