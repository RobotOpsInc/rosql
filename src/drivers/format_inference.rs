//! Format hint inference — maps query shape to a presentation-layer `FormatHint`.
//!
//! This is a pure function with no side effects. It examines the parsed AST
//! and returns the most appropriate `FormatHint` and optional `VisualizationConfig`
//! for consumers (frontend REPL, CLI, Go planner).
//!
//! Inference order:
//! 1. If an explicit `FORMAT` clause is present, map it to `FormatHint`.
//! 2. Otherwise apply the rule table below.
//!
//! | Query pattern                         | format_hint        |
//! |---------------------------------------|--------------------|
//! | TIMESERIES + FACET                    | StackedLineChart   |
//! | TIMESERIES (no FACET)                 | LineChart          |
//! | FACET (no TIMESERIES)                 | BarChart           |
//! | TRACE 'id'                            | Gantt              |
//! | MESSAGE FLOW                          | DirectedGraph      |
//! | SHOW NODE GRAPH                       | NodeGraph          |
//! | SHOW SPAN SUMMARY                     | HorizontalBars     |
//! | ANOMALY(...)                          | Table (colored)    |
//! | Scalar agg only (no GROUP BY/FACET)   | ScalarCards        |
//! | FROM logs / FROM system_logs          | LogTable           |
//! | FROM recordings / SHOW RECORDING      | RecordingList      |
//! | Everything else                       | Table              |

use crate::ast::{
    AggregationFn, CompoundClause, DataSource, FormatHint, OutputFormat, PipelineStage, Query,
    Selection,
};

use super::VisualizationConfig;

/// Infer the presentation-layer `FormatHint` and optional `VisualizationConfig`
/// from a parsed ROSQL query.
pub fn infer_format(query: &Query) -> (FormatHint, Option<VisualizationConfig>) {
    match query {
        Query::Standard(sq) => {
            // Explicit FORMAT clause overrides inference.
            if let Some(fmt) = sq.output_format {
                return (output_format_to_hint(fmt), None);
            }
            infer_standard(sq)
        }
        Query::Pipeline(pq) => {
            // Check for explicit FORMAT stage first.
            for stage in &pq.stages {
                if let PipelineStage::Format(fmt) = stage {
                    return (output_format_to_hint(*fmt), None);
                }
            }
            infer_pipeline(pq)
        }
        Query::Compound(cq) => {
            if let Some(fmt) = cq.output_format {
                return (output_format_to_hint(fmt), None);
            }
            infer_compound(cq)
        }
    }
}

fn infer_standard(sq: &crate::ast::ROSQLQuery) -> (FormatHint, Option<VisualizationConfig>) {
    let has_timeseries = sq.timeseries.is_some();
    let has_facet = sq.facet.is_some();

    if has_timeseries && has_facet {
        let facet_dim = sq.facet.as_ref().map(|f| f.dimension.clone());
        let y_axis = first_agg_alias(&sq.selections);
        return (
            FormatHint::StackedLineChart,
            Some(VisualizationConfig {
                x_axis: Some("time_bucket".into()),
                y_axis,
                series_key: facet_dim,
                color_field: None,
                label_field: None,
            }),
        );
    }

    if has_timeseries {
        let y_axis = first_agg_alias(&sq.selections);
        return (
            FormatHint::LineChart,
            Some(VisualizationConfig {
                x_axis: Some("time_bucket".into()),
                y_axis,
                series_key: None,
                color_field: None,
                label_field: None,
            }),
        );
    }

    if has_facet {
        let facet_dim = sq.facet.as_ref().map(|f| f.dimension.clone());
        let y_axis = first_agg_alias(&sq.selections);
        return (
            FormatHint::BarChart,
            Some(VisualizationConfig {
                x_axis: facet_dim,
                y_axis,
                series_key: None,
                color_field: None,
                label_field: None,
            }),
        );
    }

    // Scalar cards: only aggregations in SELECT, no GROUP BY, no FACET.
    if is_pure_scalar(&sq.selections) {
        let label = first_agg_alias(&sq.selections);
        return (
            FormatHint::ScalarCards,
            Some(VisualizationConfig {
                x_axis: None,
                y_axis: None,
                series_key: None,
                color_field: None,
                label_field: label,
            }),
        );
    }

    // Log viewer for log sources.
    if matches!(sq.data_source, DataSource::Logs | DataSource::SystemLogs) {
        return (
            FormatHint::LogTable,
            Some(VisualizationConfig {
                x_axis: None,
                y_axis: None,
                series_key: None,
                color_field: Some("severity".into()),
                label_field: None,
            }),
        );
    }

    // Recording list.
    if matches!(sq.data_source, DataSource::Recordings) {
        return (FormatHint::RecordingList, None);
    }

    (FormatHint::Table, None)
}

fn infer_pipeline(pq: &crate::ast::PipelineQuery) -> (FormatHint, Option<VisualizationConfig>) {
    let has_timeseries = pq.stages.iter().any(|s| matches!(s, PipelineStage::Timeseries(_)));
    let facet_stage = pq.stages.iter().find_map(|s| {
        if let PipelineStage::Facet(f) = s {
            Some(f)
        } else {
            None
        }
    });
    let has_facet = facet_stage.is_some();

    let data_source = pq.stages.iter().find_map(|s| {
        if let PipelineStage::From(ds) = s {
            Some(ds)
        } else {
            None
        }
    });

    if has_timeseries && has_facet {
        return (
            FormatHint::StackedLineChart,
            Some(VisualizationConfig {
                x_axis: Some("time_bucket".into()),
                y_axis: None,
                series_key: facet_stage.map(|f| f.dimension.clone()),
                color_field: None,
                label_field: None,
            }),
        );
    }

    if has_timeseries {
        return (
            FormatHint::LineChart,
            Some(VisualizationConfig {
                x_axis: Some("time_bucket".into()),
                y_axis: None,
                series_key: None,
                color_field: None,
                label_field: None,
            }),
        );
    }

    if has_facet {
        return (
            FormatHint::BarChart,
            Some(VisualizationConfig {
                x_axis: facet_stage.map(|f| f.dimension.clone()),
                y_axis: None,
                series_key: None,
                color_field: None,
                label_field: None,
            }),
        );
    }

    if let Some(ds) = data_source {
        if matches!(ds, DataSource::Logs | DataSource::SystemLogs) {
            return (
                FormatHint::LogTable,
                Some(VisualizationConfig {
                    x_axis: None,
                    y_axis: None,
                    series_key: None,
                    color_field: Some("severity".into()),
                    label_field: None,
                }),
            );
        }
        if matches!(ds, DataSource::Recordings) {
            return (FormatHint::RecordingList, None);
        }
    }

    (FormatHint::Table, None)
}

fn infer_compound(cq: &crate::ast::CompoundQuery) -> (FormatHint, Option<VisualizationConfig>) {
    match &cq.clause {
        CompoundClause::Trace { .. } => (FormatHint::Gantt, None),

        CompoundClause::MessageFlow { .. } => (FormatHint::DirectedGraph, None),

        CompoundClause::ShowNodeGraph => (FormatHint::NodeGraph, None),

        CompoundClause::ShowSpanSummary => (
            FormatHint::HorizontalBars,
            Some(VisualizationConfig {
                x_axis: Some("span_name".into()),
                y_axis: Some("duration".into()),
                series_key: None,
                color_field: None,
                label_field: None,
            }),
        ),

        CompoundClause::Anomaly { .. } => (
            FormatHint::Table,
            Some(VisualizationConfig {
                x_axis: None,
                y_axis: None,
                series_key: None,
                color_field: Some("is_anomalous".into()),
                label_field: None,
            }),
        ),

        CompoundClause::Health => (FormatHint::ScalarCards, None),

        CompoundClause::ShowRecording => (FormatHint::RecordingList, None),

        CompoundClause::PathDeviation { .. } => (
            FormatHint::LineChart,
            Some(VisualizationConfig {
                x_axis: Some("waypoint_index".into()),
                y_axis: Some("lateral_deviation_m".into()),
                series_key: None,
                color_field: None,
                label_field: None,
            }),
        ),

        CompoundClause::JointDeviation { .. } => (
            FormatHint::BarChart,
            Some(VisualizationConfig {
                x_axis: Some("joint_name".into()),
                y_axis: Some("position_error_rad".into()),
                series_key: None,
                color_field: None,
                label_field: None,
            }),
        ),

        _ => (FormatHint::Table, None),
    }
}

/// Map an explicit `OutputFormat` clause to the closest `FormatHint`.
fn output_format_to_hint(fmt: OutputFormat) -> FormatHint {
    match fmt {
        OutputFormat::Table => FormatHint::Table,
        OutputFormat::Timeseries => FormatHint::LineChart,
        OutputFormat::Scalar => FormatHint::ScalarCards,
        OutputFormat::TraceTree => FormatHint::Gantt,
        OutputFormat::Graph => FormatHint::DirectedGraph,
        OutputFormat::Path => FormatHint::LineChart,
    }
}

/// Return the alias of the first aggregation in a selection list, if any.
fn first_agg_alias(selections: &[Selection]) -> Option<String> {
    for sel in selections {
        match sel {
            Selection::Aliased { alias, expr } => {
                if matches!(expr.as_ref(), Selection::Aggregation(_)) {
                    return Some(alias.clone());
                }
            }
            Selection::Aggregation(agg) => {
                return Some(format!("{:?}", agg.function).to_lowercase());
            }
            _ => {}
        }
    }
    None
}

/// Return true when all non-star selections are aggregations (no plain field refs).
fn is_pure_scalar(selections: &[Selection]) -> bool {
    if selections.is_empty() {
        return false;
    }
    selections.iter().all(|s| match s {
        Selection::Aggregation(_) => true,
        Selection::Aliased { expr, .. } => matches!(expr.as_ref(), Selection::Aggregation(_)),
        _ => false,
    })
}

/// Check whether a selection contains any non-trivial aggregation.
#[allow(dead_code)]
fn has_aggregation(selections: &[Selection]) -> bool {
    selections.iter().any(|s| match s {
        Selection::Aggregation(a) => !matches!(a.function, AggregationFn::Count),
        Selection::Aliased { expr, .. } => {
            matches!(expr.as_ref(), Selection::Aggregation(_))
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn hint(q: &str) -> FormatHint {
        let ast = parse(q).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
        infer_format(&ast).0
    }

    fn viz(q: &str) -> Option<VisualizationConfig> {
        let ast = parse(q).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
        infer_format(&ast).1
    }

    #[test]
    fn timeseries_no_facet_is_line_chart() {
        assert_eq!(
            hint("SELECT COUNT(*) FROM traces TIMESERIES 5 min SINCE 1 hour ago"),
            FormatHint::LineChart
        );
    }

    #[test]
    fn timeseries_with_facet_is_stacked_line_chart() {
        assert_eq!(
            hint("SELECT COUNT(*) FROM traces TIMESERIES 5 min FACET robot_id SINCE 1 hour ago"),
            FormatHint::StackedLineChart
        );
    }

    #[test]
    fn stacked_viz_has_series_key() {
        let v = viz("SELECT COUNT(*) FROM traces TIMESERIES 5 min FACET robot_id SINCE 1 hour ago")
            .expect("expected viz");
        assert_eq!(v.series_key.as_deref(), Some("robot_id"));
        assert_eq!(v.x_axis.as_deref(), Some("time_bucket"));
    }

    #[test]
    fn facet_without_timeseries_is_bar_chart() {
        assert_eq!(
            hint("SELECT COUNT(*) FROM traces FACET action_name SINCE 1 hour ago"),
            FormatHint::BarChart
        );
    }

    #[test]
    fn trace_command_is_gantt() {
        assert_eq!(hint("TRACE 'abc123'"), FormatHint::Gantt);
    }

    #[test]
    fn message_flow_is_directed_graph() {
        assert_eq!(hint("MESSAGE FLOW FROM TOPIC '/cmd_vel'"), FormatHint::DirectedGraph);
    }

    #[test]
    fn show_node_graph_is_node_graph() {
        assert_eq!(hint("SHOW NODE GRAPH"), FormatHint::NodeGraph);
    }

    #[test]
    fn show_span_summary_is_horizontal_bars() {
        assert_eq!(hint("SHOW SPAN SUMMARY SINCE 1 hour ago"), FormatHint::HorizontalBars);
    }

    #[test]
    fn anomaly_is_table_with_color_field() {
        assert_eq!(
            hint("ANOMALY(duration) COMPARED TO last week FACET robot_id"),
            FormatHint::Table
        );
        let v = viz("ANOMALY(duration) COMPARED TO last week FACET robot_id").unwrap();
        assert_eq!(v.color_field.as_deref(), Some("is_anomalous"));
    }

    #[test]
    fn scalar_aggregation_is_scalar_cards() {
        assert_eq!(
            hint("SELECT COUNT(*) AS total_errors, AVG(duration) AS avg_duration FROM traces SINCE 1 hour ago"),
            FormatHint::ScalarCards
        );
    }

    #[test]
    fn from_logs_is_log_table() {
        assert_eq!(hint("FROM logs WHERE severity = 'ERROR'"), FormatHint::LogTable);
    }

    #[test]
    fn log_viz_has_color_field() {
        let v = viz("FROM logs").unwrap();
        assert_eq!(v.color_field.as_deref(), Some("severity"));
    }

    #[test]
    fn from_recordings_is_recording_list() {
        assert_eq!(hint("FROM recordings"), FormatHint::RecordingList);
    }

    #[test]
    fn format_clause_overrides_inference() {
        // TIMESERIES would normally infer LineChart, but explicit FORMAT table overrides it.
        assert_eq!(
            hint("SELECT COUNT(*) FROM traces TIMESERIES 5 min FORMAT table"),
            FormatHint::Table
        );
    }

    #[test]
    fn path_deviation_is_line_chart() {
        assert_eq!(
            hint("PATH DEVIATION FOR TRACE 'abc123'"),
            FormatHint::LineChart
        );
    }

    #[test]
    fn joint_deviation_is_bar_chart() {
        assert_eq!(
            hint("JOINT DEVIATION FOR TRACE 'abc123'"),
            FormatHint::BarChart
        );
    }
}
