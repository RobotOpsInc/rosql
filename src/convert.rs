//! Conversion between native Rust AST types and proto AST types.
//!
//! Used by the gRPC sidecar, CLI, and WASM to serialize parsed ASTs.

use crate::ast;
use crate::error::ROSQLError;
use crate::proto::rosql_v1 as pb;
use crate::span::SourceLocation;

// ===========================================================================
// Query → Proto
// ===========================================================================

pub fn query_to_proto(query: &ast::Query) -> pb::RosqlQuery {
    let query_oneof = match query {
        ast::Query::Standard(sq) => pb::rosql_query::Query::Standard(standard_query_to_proto(sq)),
        ast::Query::Pipeline(pq) => pb::rosql_query::Query::Pipeline(pipeline_query_to_proto(pq)),
        ast::Query::Compound(cq) => pb::rosql_query::Query::Compound(compound_query_to_proto(cq)),
    };
    pb::RosqlQuery {
        query: Some(query_oneof),
    }
}

fn standard_query_to_proto(sq: &ast::ROSQLQuery) -> pb::StandardQuery {
    pb::StandardQuery {
        selections: sq.selections.iter().map(selection_to_proto).collect(),
        data_source: Some(data_source_to_proto(&sq.data_source)),
        scope: sq.scope.as_ref().map(query_scope_to_proto),
        conditions: sq.conditions.as_ref().map(condition_to_proto),
        facet: sq.facet.as_ref().map(facet_to_proto),
        time_range: sq.time_range.as_ref().map(time_range_to_proto),
        time_basis: sq
            .time_basis
            .map(|tb| time_basis_to_proto(tb) as i32)
            .unwrap_or(0),
        order_by: sq.order_by.as_ref().map(order_by_to_proto),
        limit: sq.limit,
        offset: sq.offset,
        output_format: sq
            .output_format
            .map(|f| output_format_to_proto(f) as i32)
            .unwrap_or(0),
        baseline: sq.baseline.as_ref().map(baseline_to_proto),
        timeseries: sq.timeseries.as_ref().map(timeseries_to_proto),
        enrichments: sq.enrichments.iter().map(enrichment_to_proto).collect(),
    }
}

fn unit_value_to_proto(uv: &ast::UnitValue) -> pb::UnitValue {
    pb::UnitValue {
        raw_value: uv.raw_value,
        unit: uv.unit.clone(),
        si_value: uv.si_value,
        si_unit: uv.si_unit.clone(),
    }
}

fn timeseries_to_proto(ts: &ast::TimeseriesClause) -> pb::TimeseriesClause {
    pb::TimeseriesClause {
        interval: Some(unit_value_to_proto(&ts.interval)),
    }
}

fn enrichment_to_proto(e: &ast::EnrichmentClause) -> pb::EnrichmentClause {
    pb::EnrichmentClause {
        source: Some(data_source_to_proto(&e.source)),
        join_key: e.join_key.clone().unwrap_or_default(),
        limit: e.limit,
        sample_full: e.sample_full,
    }
}

fn pipeline_query_to_proto(pq: &ast::PipelineQuery) -> pb::PipelineQuery {
    pb::PipelineQuery {
        stages: pq.stages.iter().map(pipeline_stage_to_proto).collect(),
    }
}

fn pipeline_stage_to_proto(stage: &ast::PipelineStage) -> pb::PipelineStage {
    let stage_oneof = match stage {
        ast::PipelineStage::From(ds) => pb::pipeline_stage::Stage::From(data_source_to_proto(ds)),
        ast::PipelineStage::Select(sels) => pb::pipeline_stage::Stage::Select(pb::SelectStage {
            selections: sels.iter().map(selection_to_proto).collect(),
        }),
        ast::PipelineStage::Where(cond) => {
            pb::pipeline_stage::Stage::Where(condition_to_proto(cond))
        }
        ast::PipelineStage::Facet(f) => pb::pipeline_stage::Stage::Facet(facet_to_proto(f)),
        ast::PipelineStage::Since(tr) => pb::pipeline_stage::Stage::Since(time_range_to_proto(tr)),
        ast::PipelineStage::Using(tb) => {
            pb::pipeline_stage::Stage::Using(time_basis_to_proto(*tb) as i32)
        }
        ast::PipelineStage::OrderBy(ob) => {
            pb::pipeline_stage::Stage::OrderBy(order_by_to_proto(ob))
        }
        ast::PipelineStage::Limit(n) => pb::pipeline_stage::Stage::Limit(*n),
        ast::PipelineStage::Offset(n) => pb::pipeline_stage::Stage::Offset(*n),
        ast::PipelineStage::Format(f) => {
            pb::pipeline_stage::Stage::Format(output_format_to_proto(*f) as i32)
        }
        ast::PipelineStage::CompareTo(b) => {
            pb::pipeline_stage::Stage::CompareTo(baseline_to_proto(b))
        }
        ast::PipelineStage::ForScope(s) => {
            pb::pipeline_stage::Stage::ForScope(query_scope_to_proto(s))
        }
        ast::PipelineStage::CompoundClause(cc) => {
            pb::pipeline_stage::Stage::CompoundClause(compound_clause_to_proto(cc))
        }
        ast::PipelineStage::Timeseries(ts) => {
            pb::pipeline_stage::Stage::Timeseries(timeseries_to_proto(ts))
        }
        ast::PipelineStage::EnrichWith(e) => {
            pb::pipeline_stage::Stage::EnrichWith(enrichment_to_proto(e))
        }
    };
    pb::PipelineStage {
        stage: Some(stage_oneof),
    }
}

fn compound_query_to_proto(cq: &ast::CompoundQuery) -> pb::CompoundQuery {
    pb::CompoundQuery {
        clause: Some(compound_clause_to_proto(&cq.clause)),
        scope: cq.scope.as_ref().map(query_scope_to_proto),
        time_range: cq.time_range.as_ref().map(time_range_to_proto),
        time_basis: cq
            .time_basis
            .map(|tb| time_basis_to_proto(tb) as i32)
            .unwrap_or(0),
        conditions: cq.conditions.as_ref().map(condition_to_proto),
        facet: cq.facet.as_ref().map(facet_to_proto),
        order_by: cq.order_by.as_ref().map(order_by_to_proto),
        limit: cq.limit,
        offset: cq.offset,
        output_format: cq
            .output_format
            .map(|f| output_format_to_proto(f) as i32)
            .unwrap_or(0),
        baseline: cq.baseline.as_ref().map(baseline_to_proto),
    }
}

// ===========================================================================
// Component converters
// ===========================================================================

fn data_source_to_proto(ds: &ast::DataSource) -> pb::DataSource {
    let source = match ds {
        ast::DataSource::Logs => pb::data_source::Source::Type(pb::DataSourceType::Logs as i32),
        ast::DataSource::SystemLogs => {
            pb::data_source::Source::Type(pb::DataSourceType::SystemLogs as i32)
        }
        ast::DataSource::Traces => pb::data_source::Source::Type(pb::DataSourceType::Traces as i32),
        ast::DataSource::Metrics => {
            pb::data_source::Source::Type(pb::DataSourceType::Metrics as i32)
        }
        ast::DataSource::Diagnostics => {
            pb::data_source::Source::Type(pb::DataSourceType::Diagnostics as i32)
        }
        ast::DataSource::Topics => pb::data_source::Source::Type(pb::DataSourceType::Topics as i32),
        ast::DataSource::Tf => pb::data_source::Source::Type(pb::DataSourceType::Tf as i32),
        ast::DataSource::Heartbeats => {
            pb::data_source::Source::Type(pb::DataSourceType::Heartbeats as i32)
        }
        ast::DataSource::Recordings => {
            pb::data_source::Source::Type(pb::DataSourceType::Recordings as i32)
        }
        ast::DataSource::Events => pb::data_source::Source::Type(pb::DataSourceType::Events as i32),
        ast::DataSource::TopicAlias(alias) => {
            let alias_type = match alias {
                ast::TopicAlias::Odom => pb::TopicAliasType::Odom,
                ast::TopicAlias::JointStates => pb::TopicAliasType::JointStates,
                ast::TopicAlias::Battery => pb::TopicAliasType::Battery,
                ast::TopicAlias::CmdVel => pb::TopicAliasType::CmdVel,
                ast::TopicAlias::Imu => pb::TopicAliasType::Imu,
            };
            pb::data_source::Source::TopicAlias(alias_type as i32)
        }
    };
    pb::DataSource {
        source: Some(source),
    }
}

fn selection_to_proto(sel: &ast::Selection) -> pb::Selection {
    let selection = match sel {
        ast::Selection::Star => pb::selection::Selection::Star(true),
        ast::Selection::Field(name) => pb::selection::Selection::Field(name.clone()),
        ast::Selection::Aggregation(agg) => {
            pb::selection::Selection::Aggregation(aggregation_to_proto(agg))
        }
        ast::Selection::Aliased { expr, alias } => {
            pb::selection::Selection::Aliased(Box::new(pb::AliasedSelection {
                expr: Some(Box::new(selection_to_proto(expr))),
                alias: alias.clone(),
            }))
        }
    };
    pb::Selection {
        selection: Some(selection),
    }
}

fn aggregation_to_proto(agg: &ast::AggregationCall) -> pb::AggregationCall {
    let function = match agg.function {
        ast::AggregationFn::Count => pb::AggregationFunction::Count,
        ast::AggregationFn::Sum => pb::AggregationFunction::Sum,
        ast::AggregationFn::Avg => pb::AggregationFunction::Avg,
        ast::AggregationFn::Min => pb::AggregationFunction::Min,
        ast::AggregationFn::Max => pb::AggregationFunction::Max,
        ast::AggregationFn::Percentile => pb::AggregationFunction::Percentile,
        ast::AggregationFn::Stddev => pb::AggregationFunction::Stddev,
        ast::AggregationFn::Rate => pb::AggregationFunction::Rate,
        ast::AggregationFn::Delta => pb::AggregationFunction::Delta,
        ast::AggregationFn::Derivative => pb::AggregationFunction::Derivative,
        ast::AggregationFn::MovingAvg => pb::AggregationFunction::MovingAvg,
        ast::AggregationFn::TopicRate => pb::AggregationFunction::TopicRate,
        ast::AggregationFn::NodeStatus => pb::AggregationFunction::NodeStatus,
        ast::AggregationFn::Expected => pb::AggregationFunction::Expected,
        ast::AggregationFn::ActionSuccessRate => pb::AggregationFunction::ActionSuccessRate,
        ast::AggregationFn::Uptime => pb::AggregationFunction::Uptime,
        ast::AggregationFn::ApproxCountDistinct => pb::AggregationFunction::ApproxCountDistinct,
        ast::AggregationFn::ApproxPercentile => pb::AggregationFunction::ApproxPercentile,
    };
    pb::AggregationCall {
        function: function as i32,
        args: agg.args.iter().map(expr_to_proto).collect(),
    }
}

fn expr_to_proto(expr: &ast::Expr) -> pb::Expr {
    let expr_oneof = match expr {
        ast::Expr::Field(name) => pb::expr::Expr::Field(name.clone()),
        ast::Expr::Literal(lit) => pb::expr::Expr::Literal(literal_to_proto(lit)),
        ast::Expr::UnitValue(uv) => pb::expr::Expr::UnitValue(pb::UnitValue {
            raw_value: uv.raw_value,
            unit: uv.unit.clone(),
            si_value: uv.si_value,
            si_unit: uv.si_unit.clone(),
        }),
        ast::Expr::Aggregation(agg) => pb::expr::Expr::Aggregation(aggregation_to_proto(agg)),
        ast::Expr::FieldAccess { base, key } => pb::expr::Expr::FieldAccess(pb::FieldAccess {
            base: base.clone(),
            key: key.clone(),
        }),
        ast::Expr::BinaryOp { left, op, right } => {
            let op_proto = match op {
                ast::ArithmeticOp::Add => pb::ArithmeticOp::Add,
                ast::ArithmeticOp::Sub => pb::ArithmeticOp::Sub,
                ast::ArithmeticOp::Mul => pb::ArithmeticOp::Mul,
                ast::ArithmeticOp::Div => pb::ArithmeticOp::Div,
            };
            pb::expr::Expr::BinaryOp(Box::new(pb::BinaryOp {
                left: Some(Box::new(expr_to_proto(left))),
                op: op_proto as i32,
                right: Some(Box::new(expr_to_proto(right))),
            }))
        }
    };
    pb::Expr {
        expr: Some(expr_oneof),
    }
}

fn literal_to_proto(lit: &ast::Literal) -> pb::Literal {
    let value = match lit {
        ast::Literal::Integer(n) => pb::literal::Value::Integer(*n),
        ast::Literal::Float(f) => pb::literal::Value::Float(*f),
        ast::Literal::String(s) => pb::literal::Value::StringValue(s.clone()),
        ast::Literal::Boolean(b) => pb::literal::Value::Boolean(*b),
        ast::Literal::Null => pb::literal::Value::Null(true),
    };
    pb::Literal { value: Some(value) }
}

fn condition_to_proto(cond: &ast::Condition) -> pb::Condition {
    let cond_oneof = match cond {
        ast::Condition::Comparison { left, op, right } => {
            let op_proto = match op {
                ast::ComparisonOp::Eq => pb::ComparisonOp::Eq,
                ast::ComparisonOp::Neq => pb::ComparisonOp::Neq,
                ast::ComparisonOp::Lt => pb::ComparisonOp::Lt,
                ast::ComparisonOp::Gt => pb::ComparisonOp::Gt,
                ast::ComparisonOp::Lte => pb::ComparisonOp::Lte,
                ast::ComparisonOp::Gte => pb::ComparisonOp::Gte,
            };
            pb::condition::Condition::Comparison(pb::Comparison {
                left: Some(expr_to_proto(left)),
                op: op_proto as i32,
                right: Some(expr_to_proto(right)),
            })
        }
        ast::Condition::And(a, b) => pb::condition::Condition::And(Box::new(pb::LogicalOp {
            left: Some(Box::new(condition_to_proto(a))),
            right: Some(Box::new(condition_to_proto(b))),
        })),
        ast::Condition::Or(a, b) => pb::condition::Condition::Or(Box::new(pb::LogicalOp {
            left: Some(Box::new(condition_to_proto(a))),
            right: Some(Box::new(condition_to_proto(b))),
        })),
        ast::Condition::Not(inner) => {
            pb::condition::Condition::Not(Box::new(condition_to_proto(inner)))
        }
        ast::Condition::IsNull(expr) => pb::condition::Condition::IsNull(expr_to_proto(expr)),
        ast::Condition::IsNotNull(expr) => pb::condition::Condition::IsNotNull(expr_to_proto(expr)),
        ast::Condition::In { expr, values } => pb::condition::Condition::InExpr(pb::InExpr {
            expr: Some(expr_to_proto(expr)),
            values: values.iter().map(expr_to_proto).collect(),
        }),
        ast::Condition::Like { expr, pattern } => pb::condition::Condition::Like(pb::LikeExpr {
            expr: Some(expr_to_proto(expr)),
            pattern: pattern.clone(),
        }),
        ast::Condition::Between { expr, low, high } => {
            pb::condition::Condition::Between(pb::BetweenExpr {
                expr: Some(expr_to_proto(expr)),
                low: Some(expr_to_proto(low)),
                high: Some(expr_to_proto(high)),
            })
        }
        // WITHIN geospatial condition — proto not yet defined; fall back to a stub.
        // TODO: add WithinCondition to ast.proto for full proto round-trip support.
        ast::Condition::Within { .. } => pb::condition::Condition::IsNotNull(pb::Expr::default()),
    };
    pb::Condition {
        condition: Some(cond_oneof),
    }
}

fn time_range_to_proto(tr: &ast::TimeRange) -> pb::TimeRange {
    let range = match tr {
        ast::TimeRange::Since(expr) => pb::time_range::Range::Since(time_expr_to_proto(expr)),
        ast::TimeRange::Between { start, end } => pb::time_range::Range::Between(pb::BetweenTime {
            start: Some(time_expr_to_proto(start)),
            end: Some(time_expr_to_proto(end)),
        }),
    };
    pb::TimeRange { range: Some(range) }
}

fn time_expr_to_proto(expr: &ast::TimeExpr) -> pb::TimeExpr {
    let expr_oneof = match expr {
        ast::TimeExpr::Relative(rt) => pb::time_expr::Expr::Relative(pb::RelativeTime {
            amount: rt.amount,
            unit: rt.unit.clone(),
        }),
        ast::TimeExpr::Absolute(ts) => pb::time_expr::Expr::Absolute(ts.clone()),
        ast::TimeExpr::Epoch(epoch) => {
            let precision = match epoch {
                ast::UnixEpoch::Seconds(v) => pb::unix_epoch::Precision::Seconds(*v),
                ast::UnixEpoch::Milliseconds(v) => pb::unix_epoch::Precision::Milliseconds(*v),
                ast::UnixEpoch::Nanoseconds(v) => pb::unix_epoch::Precision::Nanoseconds(*v),
            };
            pb::time_expr::Expr::Epoch(pb::UnixEpoch {
                precision: Some(precision),
            })
        }
        ast::TimeExpr::Anchor(anchor) => {
            let anchor_proto = match anchor {
                ast::LifecycleAnchor::LastDeployment => pb::LifecycleAnchor::LastDeployment,
                ast::LifecycleAnchor::LastRobotRestart => pb::LifecycleAnchor::LastRobotRestart,
                ast::LifecycleAnchor::LastActionFailure => pb::LifecycleAnchor::LastActionFailure,
                ast::LifecycleAnchor::LastTopicDrop => pb::LifecycleAnchor::LastTopicDrop,
                ast::LifecycleAnchor::LastDiagnosticWarning => {
                    pb::LifecycleAnchor::LastDiagnosticWarning
                }
            };
            pb::time_expr::Expr::Anchor(anchor_proto as i32)
        }
    };
    pb::TimeExpr {
        expr: Some(expr_oneof),
    }
}

fn robot_scope_to_proto(scope: &ast::RobotScope) -> pb::RobotScope {
    let scope_oneof = match scope {
        ast::RobotScope::Single(id) => pb::robot_scope::Scope::RobotId(id.clone()),
        ast::RobotScope::Fleet => pb::robot_scope::Scope::Fleet(true),
    };
    pb::RobotScope {
        scope: Some(scope_oneof),
    }
}

fn query_scope_to_proto(scope: &ast::QueryScope) -> pb::QueryScope {
    pb::QueryScope {
        robot: scope.robot.as_ref().map(robot_scope_to_proto),
        version: scope.version.clone().unwrap_or_default(),
        environment: scope.environment.clone().unwrap_or_default(),
        session: scope.session.clone().unwrap_or_default(),
    }
}

fn facet_to_proto(f: &ast::FacetClause) -> pb::FacetClause {
    pb::FacetClause {
        dimension: f.dimension.clone(),
    }
}

fn order_by_to_proto(ob: &ast::OrderBy) -> pb::OrderBy {
    let dir = match ob.direction {
        ast::SortDirection::Asc => pb::SortDirection::Asc,
        ast::SortDirection::Desc => pb::SortDirection::Desc,
    };
    pb::OrderBy {
        field: ob.field.clone(),
        direction: dir as i32,
    }
}

fn time_basis_to_proto(tb: ast::TimeBasis) -> pb::TimeBasis {
    match tb {
        ast::TimeBasis::RosTime => pb::TimeBasis::RosTime,
        ast::TimeBasis::WallTime => pb::TimeBasis::WallTime,
    }
}

fn output_format_to_proto(f: ast::OutputFormat) -> pb::OutputFormat {
    match f {
        ast::OutputFormat::Table => pb::OutputFormat::Table,
        ast::OutputFormat::Timeseries => pb::OutputFormat::Timeseries,
        ast::OutputFormat::Scalar => pb::OutputFormat::Scalar,
        ast::OutputFormat::TraceTree => pb::OutputFormat::TraceTree,
        ast::OutputFormat::Graph => pb::OutputFormat::Graph,
        ast::OutputFormat::Path => pb::OutputFormat::Path,
    }
}

/// Convert a `FormatHint` to its proto enum integer value.
///
/// The proto `FormatHint` enum is defined in `result.proto` but not yet
/// code-generated (we use the raw integer constants here).
pub fn format_hint_to_proto_int(hint: ast::FormatHint) -> i32 {
    match hint {
        ast::FormatHint::Table => 0,
        ast::FormatHint::LineChart => 1,
        ast::FormatHint::StackedLineChart => 2,
        ast::FormatHint::BarChart => 3,
        ast::FormatHint::HorizontalBars => 4,
        ast::FormatHint::Gantt => 5,
        ast::FormatHint::DirectedGraph => 6,
        ast::FormatHint::NodeGraph => 7,
        ast::FormatHint::ScalarCards => 8,
        ast::FormatHint::LogTable => 9,
        ast::FormatHint::RecordingList => 10,
    }
}

fn baseline_to_proto(b: &ast::Baseline) -> pb::Baseline {
    let baseline = match b {
        ast::Baseline::LastWeek => pb::baseline::Baseline::LastWeek(true),
        // Last24Hours not yet in proto — map to LastWeek as a stub until proto is updated.
        ast::Baseline::Last24Hours => pb::baseline::Baseline::LastWeek(true),
        ast::Baseline::Fleet => pb::baseline::Baseline::Fleet(true),
        ast::Baseline::Robot(id) => pb::baseline::Baseline::RobotId(id.clone()),
        ast::Baseline::LastDeployment => pb::baseline::Baseline::LastDeployment(true),
        ast::Baseline::CompareRobots => pb::baseline::Baseline::CompareRobots(true),
        ast::Baseline::Version(v) => pb::baseline::Baseline::Version(v.clone()),
        ast::Baseline::VersionPair(v1, v2) => {
            pb::baseline::Baseline::VersionPair(pb::VersionPairBaseline {
                from_version: v1.clone(),
                to_version: v2.clone(),
            })
        }
    };
    pb::Baseline {
        baseline: Some(baseline),
    }
}

fn compound_clause_to_proto(cc: &ast::CompoundClause) -> pb::CompoundClause {
    let clause = match cc {
        ast::CompoundClause::During {
            inner_source,
            inner_conditions,
            inner_time_range,
        } => pb::compound_clause::Clause::During(pb::DuringClause {
            inner_source: Some(data_source_to_proto(inner_source)),
            inner_conditions: inner_conditions.as_ref().map(condition_to_proto),
            inner_time_range: inner_time_range.as_ref().map(time_range_to_proto),
        }),
        ast::CompoundClause::Trace { trace_id } => {
            pb::compound_clause::Clause::Trace(pb::TraceClause {
                trace_id: trace_id.clone(),
            })
        }
        ast::CompoundClause::MessageFlow {
            from_topic,
            to_target,
            show,
        } => {
            let to_target_proto = to_target.as_ref().map(|t| {
                let target = match t {
                    ast::FlowTarget::Node(n) => pb::flow_target::Target::Node(n.clone()),
                    ast::FlowTarget::Topic(t) => pb::flow_target::Target::Topic(t.clone()),
                };
                pb::FlowTarget {
                    target: Some(target),
                }
            });
            pb::compound_clause::Clause::MessageFlow(pb::MessageFlowClause {
                from_topic: from_topic.clone(),
                to_target: to_target_proto,
                show: show.clone().unwrap_or_default(),
            })
        }
        ast::CompoundClause::ShowDeployments => pb::compound_clause::Clause::ShowDeployments(true),
        ast::CompoundClause::ShowSpanSummary => pb::compound_clause::Clause::ShowSpanSummary(true),
        ast::CompoundClause::ShowPlans { trace_id } => {
            pb::compound_clause::Clause::ShowPlans(pb::ShowPlansClause {
                trace_id: trace_id.clone().unwrap_or_default(),
            })
        }
        ast::CompoundClause::ShowTraceBreakdown => {
            pb::compound_clause::Clause::ShowTraceBreakdown(true)
        }
        ast::CompoundClause::Health => pb::compound_clause::Clause::Health(true),
        ast::CompoundClause::Anomaly {
            field, compared_to, ..
        } => {
            pb::compound_clause::Clause::Anomaly(pb::AnomalyClause {
                field: field.clone(),
                // compared_to is now required; wrap in Some for proto (which expects optional)
                compared_to: Some(baseline_to_proto(compared_to)),
            })
        }
        ast::CompoundClause::PathDeviation { .. } => {
            // Proto PathDeviationClause not yet updated to match new AST shape.
            // Use ShowRecording as a placeholder until proto is updated.
            pb::compound_clause::Clause::PathDeviation(pb::PathDeviationClause { show: vec![] })
        }
        ast::CompoundClause::JointDeviation { .. } => {
            // Not yet in proto — stub as ShowRecording until proto is updated.
            pb::compound_clause::Clause::ShowRecording(true)
        }
        ast::CompoundClause::Correlate { with_source } => {
            pb::compound_clause::Clause::Correlate(pb::CorrelateClause {
                with_source: Some(data_source_to_proto(with_source)),
            })
        }
        ast::CompoundClause::ShowRecording => pb::compound_clause::Clause::ShowRecording(true),
        ast::CompoundClause::ShowTopics => pb::compound_clause::Clause::ShowTopics(true),
        ast::CompoundClause::ShowNodes => pb::compound_clause::Clause::ShowNodes(true),
        ast::CompoundClause::ShowNodeGraph => pb::compound_clause::Clause::ShowNodeGraph(true),
        // ShowJoints not yet in proto — stub as ShowTopics until proto is updated.
        ast::CompoundClause::ShowJoints => pb::compound_clause::Clause::ShowTopics(true),
    };
    pb::CompoundClause {
        clause: Some(clause),
    }
}

// ===========================================================================
// Error conversion
// ===========================================================================

pub fn error_to_proto(err: &ROSQLError) -> pb::ParseErrorDetail {
    match err {
        ROSQLError::ParseError {
            message,
            location,
            suggestion,
        } => pb::ParseErrorDetail {
            message: message.clone(),
            location: Some(source_location_to_proto(location)),
            suggestion: suggestion.clone().unwrap_or_default(),
        },
        other => pb::ParseErrorDetail {
            message: other.to_string(),
            location: None,
            suggestion: String::new(),
        },
    }
}

pub fn source_location_to_proto(loc: &SourceLocation) -> pb::SourceLocation {
    pb::SourceLocation {
        line: loc.line as u32,
        column: loc.column as u32,
        offset: loc.offset as u32,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn convert_basic_select() {
        let ast = parse("SELECT * FROM logs").unwrap();
        let proto = query_to_proto(&ast);
        assert!(proto.query.is_some());
        match proto.query.unwrap() {
            pb::rosql_query::Query::Standard(sq) => {
                assert_eq!(sq.selections.len(), 1);
                assert!(sq.data_source.is_some());
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn convert_pipeline() {
        let ast = parse("FROM traces | WHERE duration > 500 ms | FACET robot_id").unwrap();
        let proto = query_to_proto(&ast);
        match proto.query.unwrap() {
            pb::rosql_query::Query::Pipeline(pq) => {
                assert_eq!(pq.stages.len(), 3);
            }
            _ => panic!("expected Pipeline"),
        }
    }

    #[test]
    fn convert_compound_health() {
        let ast = parse("HEALTH() FOR ROBOT 'robot_42' SINCE 1 hour ago").unwrap();
        let proto = query_to_proto(&ast);
        match proto.query.unwrap() {
            pb::rosql_query::Query::Compound(cq) => {
                assert!(cq.clause.is_some());
                assert!(cq.scope.is_some());
                assert!(cq.time_range.is_some());
            }
            _ => panic!("expected Compound"),
        }
    }

    #[test]
    fn convert_unit_value() {
        let ast = parse("FROM traces WHERE duration > 500 ms").unwrap();
        let proto = query_to_proto(&ast);
        // Verify the unit value made it through
        match proto.query.unwrap() {
            pb::rosql_query::Query::Standard(sq) => {
                assert!(sq.conditions.is_some());
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn convert_error() {
        let err = ROSQLError::ParseError {
            message: "unexpected token".into(),
            location: SourceLocation {
                line: 1,
                column: 5,
                offset: 4,
            },
            suggestion: Some("SELECT".into()),
        };
        let proto = error_to_proto(&err);
        assert_eq!(proto.message, "unexpected token");
        assert_eq!(proto.suggestion, "SELECT");
        let loc = proto.location.unwrap();
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn convert_lifecycle_anchor() {
        let ast = parse("FROM traces SINCE last action failure").unwrap();
        let proto = query_to_proto(&ast);
        match proto.query.unwrap() {
            pb::rosql_query::Query::Standard(sq) => {
                assert!(sq.time_range.is_some());
            }
            _ => panic!("expected Standard"),
        }
    }

    #[test]
    fn convert_trace() {
        let ast = parse("TRACE 'abc123'").unwrap();
        let proto = query_to_proto(&ast);
        match proto.query.unwrap() {
            pb::rosql_query::Query::Compound(cq) => {
                let clause = cq.clause.unwrap().clause.unwrap();
                match clause {
                    pb::compound_clause::Clause::Trace(t) => {
                        assert_eq!(t.trace_id, "abc123");
                    }
                    _ => panic!("expected Trace"),
                }
            }
            _ => panic!("expected Compound"),
        }
    }
}
