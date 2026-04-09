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

// ── TIMESERIES pipeline stage ────────────────────────────────────────────────

#[test]
fn snapshot_pipeline_timeseries() {
    let ast = parse("FROM traces | WHERE status = 'ERROR' | TIMESERIES 5 min | SINCE 6 hours ago")
        .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_pipeline_timeseries_with_facet() {
    let ast = parse(
        "FROM traces | SELECT AVG(duration) | TIMESERIES 1 min | FACET action_name | SINCE 1 hour ago",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

// ── ENRICH WITH pipeline stage ───────────────────────────────────────────────

#[test]
fn snapshot_pipeline_enrich_with() {
    let ast = parse("FROM traces | WHERE status = 'ERROR' | ENRICH WITH logs | SINCE 1 hour ago")
        .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_pipeline_enrich_with_limit() {
    let ast = parse(
        "FROM traces | WHERE status = 'ERROR' | ENRICH WITH logs LIMIT 200 | SINCE 1 hour ago",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}

#[test]
fn snapshot_pipeline_enrich_multiple() {
    let ast = parse(
        "FROM traces | WHERE status = 'ERROR' | ENRICH WITH logs | ENRICH WITH recordings | SINCE 1 hour ago",
    )
    .unwrap();
    insta::assert_yaml_snapshot!(ast);
}
