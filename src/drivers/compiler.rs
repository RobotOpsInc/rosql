//! AST → SQL compilation.
//!
//! Pure functions that transform a parsed ROSQL AST into a SQL string.
//! No database connection is needed — this module is fully testable in isolation.

use crate::ast::*;
use crate::error::ROSQLError;

use super::dialect::SqlDialect;
use super::field_registry::FieldRegistry;
use super::BackendCapabilities;

/// Compile a ROSQL AST to a SQL string.
pub fn compile(
    query: &Query,
    registry: &FieldRegistry,
    dialect: &SqlDialect,
    capabilities: &BackendCapabilities,
) -> Result<String, ROSQLError> {
    let ctx = CompileCtx {
        registry,
        dialect,
        capabilities,
    };
    match query {
        Query::Standard(sq) => ctx.compile_standard(sq),
        Query::Pipeline(pq) => ctx.compile_pipeline(pq),
        Query::Compound(cq) => ctx.compile_compound(cq),
    }
}

// ---------------------------------------------------------------------------
// Compilation context
// ---------------------------------------------------------------------------

struct CompileCtx<'a> {
    registry: &'a FieldRegistry,
    dialect: &'a SqlDialect,
    capabilities: &'a BackendCapabilities,
}

impl<'a> CompileCtx<'a> {
    // ── Standard query ──────────────────────────────────────────────

    fn compile_standard(&self, q: &ROSQLQuery) -> Result<String, ROSQLError> {
        let table = self.resolve_table(&q.data_source)?;
        let mut parts = Vec::new();

        // SELECT
        let select_clause = self.compile_selections(&q.selections, &table)?;
        parts.push(format!("SELECT {select_clause}"));

        // FROM (with topic alias filter)
        let from_clause = self.compile_from(&q.data_source, &table)?;
        parts.push(from_clause);

        // WHERE (conditions + time range + topic alias filter combined)
        let mut where_parts = Vec::new();
        if let Some(ref cond) = q.conditions {
            where_parts.push(self.compile_condition(cond, &table)?);
        }
        if let Some(ref tr) = q.time_range {
            where_parts.push(self.compile_time_range(tr, &table)?);
        }
        if let DataSource::TopicAlias(ref alias) = q.data_source {
            where_parts.push(format!("topic_name = '{}'", alias.topic_name()));
        }
        if !where_parts.is_empty() {
            parts.push(format!("WHERE {}", where_parts.join(" AND ")));
        }

        // GROUP BY (from FACET)
        if let Some(ref facet) = q.facet {
            let col = self.resolve_column(&facet.dimension, &table)?;
            parts.push(format!("GROUP BY {col}"));
        }

        // ORDER BY
        if let Some(ref ob) = q.order_by {
            let col = self.resolve_column(&ob.field, &table)?;
            let dir = match ob.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            parts.push(format!("ORDER BY {col} {dir}"));
        }

        // LIMIT
        if let Some(limit) = q.limit {
            parts.push(format!("LIMIT {limit}"));
        }

        Ok(parts.join(" "))
    }

    // ── Pipeline query ──────────────────────────────────────────────

    fn compile_pipeline(&self, pq: &PipelineQuery) -> Result<String, ROSQLError> {
        // Normalize pipeline to a standard query, then compile.
        let sq = self.normalize_pipeline(pq)?;
        self.compile_standard(&sq)
    }

    fn normalize_pipeline(&self, pq: &PipelineQuery) -> Result<ROSQLQuery, ROSQLError> {
        let mut sq = ROSQLQuery {
            selections: vec![Selection::Star],
            data_source: DataSource::Logs, // placeholder
            robot_scope: None,
            conditions: None,
            facet: None,
            time_range: None,
            time_basis: None,
            order_by: None,
            limit: None,
            output_format: None,
            baseline: None,
        };

        for stage in &pq.stages {
            match stage {
                PipelineStage::From(source) => sq.data_source = source.clone(),
                PipelineStage::Select(sels) => sq.selections = sels.clone(),
                PipelineStage::Where(cond) => {
                    sq.conditions = Some(match sq.conditions.take() {
                        Some(existing) => {
                            Condition::And(Box::new(existing), Box::new(cond.clone()))
                        }
                        None => cond.clone(),
                    });
                }
                PipelineStage::Facet(f) => sq.facet = Some(f.clone()),
                PipelineStage::Since(tr) => sq.time_range = Some(tr.clone()),
                PipelineStage::Using(tb) => sq.time_basis = Some(*tb),
                PipelineStage::OrderBy(ob) => sq.order_by = Some(ob.clone()),
                PipelineStage::Limit(n) => sq.limit = Some(*n),
                PipelineStage::Format(f) => sq.output_format = Some(*f),
                PipelineStage::CompareTo(b) => sq.baseline = Some(b.clone()),
                PipelineStage::ForRobot(r) => sq.robot_scope = Some(r.clone()),
                PipelineStage::CompoundClause(_) => {
                    // Compound clauses in pipeline are handled separately
                }
            }
        }

        Ok(sq)
    }

    // ── Compound query ──────────────────────────────────────────────

    fn compile_compound(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        match &cq.clause {
            CompoundClause::MessageJourney { trace_id } => {
                self.compile_message_journey(trace_id, cq)
            }
            CompoundClause::MessagePaths { topic } => self.compile_message_paths(topic, cq),
            CompoundClause::MessagePath {
                from_topic,
                to_node,
                ..
            } => self.compile_message_path(from_topic, to_node, cq),
            CompoundClause::Trace { trace_id } => self.compile_trace(trace_id),
            CompoundClause::ShowTraceBreakdown => self.compile_trace_breakdown(cq),
            CompoundClause::Health => self.compile_health(cq),
            CompoundClause::Anomaly { field, compared_to } => {
                self.compile_anomaly(field, compared_to, cq)
            }
            CompoundClause::PathDeviation { .. } => self.compile_path_deviation(cq),
            CompoundClause::Correlate { with_source } => self.compile_correlate(with_source, cq),
            CompoundClause::ShowRecording => self.compile_show_recording(cq),
            CompoundClause::During {
                inner_source,
                inner_conditions,
                inner_time_range,
            } => self.compile_during(inner_source, inner_conditions, inner_time_range, cq),
        }
    }

    fn compile_message_journey(
        &self,
        trace_id: &str,
        cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let mut sql = format!(
            "WITH RECURSIVE journey AS (\
             SELECT * FROM otel_traces WHERE TraceId = '{trace_id}' AND ParentSpanId = '' \
             UNION ALL \
             SELECT t.* FROM otel_traces t \
             JOIN journey j ON t.ParentSpanId = j.SpanId\
             ) SELECT * FROM journey"
        );
        sql.push_str(&self.compile_compound_suffix(cq)?);
        Ok(sql)
    }

    fn compile_message_paths(&self, topic: &str, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let topic_attr = self.dialect.json_access("SpanAttributes", "ros.topic");
        let mut sql = format!(
            "WITH RECURSIVE paths AS (\
             SELECT * FROM otel_traces WHERE {topic_attr} = '{topic}' \
             UNION ALL \
             SELECT t.* FROM otel_traces t \
             JOIN paths p ON t.ParentSpanId = p.SpanId\
             ) SELECT * FROM paths"
        );
        sql.push_str(&self.compile_compound_suffix(cq)?);
        Ok(sql)
    }

    fn compile_message_path(
        &self,
        from_topic: &str,
        to_node: &str,
        cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let topic_attr = self.dialect.json_access("SpanAttributes", "ros.topic");
        let node_attr = self.dialect.json_access("SpanAttributes", "ros.node");
        let mut sql = format!(
            "WITH RECURSIVE msg_path AS (\
             SELECT * FROM otel_traces WHERE {topic_attr} = '{from_topic}' \
             UNION ALL \
             SELECT t.* FROM otel_traces t \
             JOIN msg_path p ON t.ParentSpanId = p.SpanId\
             ) SELECT * FROM msg_path WHERE {node_attr} = '{to_node}'"
        );
        sql.push_str(&self.compile_compound_suffix(cq)?);
        Ok(sql)
    }

    fn compile_trace(&self, trace_id: &str) -> Result<String, ROSQLError> {
        Ok(format!(
            "SELECT * FROM otel_traces WHERE TraceId = '{trace_id}' ORDER BY Timestamp"
        ))
    }

    fn compile_trace_breakdown(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let mut sql = "SELECT SpanName, COUNT(*) AS count, \
             AVG(Duration) AS avg_duration_ns, \
             MAX(Duration) AS max_duration_ns \
             FROM otel_traces"
            .to_string();

        let mut where_parts = Vec::new();
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "otel_traces")?);
        }
        if let Some(ref cond) = cq.conditions {
            where_parts.push(self.compile_condition(cond, "otel_traces")?);
        }
        if !where_parts.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_parts.join(" AND ")));
        }
        sql.push_str(" GROUP BY SpanName ORDER BY avg_duration_ns DESC");
        Ok(sql)
    }

    fn compile_health(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let time_filter = if let Some(ref tr) = cq.time_range {
            format!(" WHERE {}", self.compile_time_range(tr, "t")?,)
        } else {
            String::new()
        };

        let facet = if let Some(ref f) = cq.facet {
            let col = &f.dimension;
            format!(", {col}")
        } else {
            String::new()
        };

        let group_by = if cq.facet.is_some() {
            let col = &cq.facet.as_ref().unwrap().dimension;
            format!(" GROUP BY signal_type, {col}")
        } else {
            " GROUP BY signal_type".to_string()
        };

        Ok(format!(
            "SELECT 'traces' AS signal_type{facet}, \
             COUNT(*) AS total, \
             SUM(CASE WHEN StatusCode = 'ERROR' THEN 1 ELSE 0 END) AS errors \
             FROM otel_traces t{time_filter}{group_by} \
             UNION ALL \
             SELECT 'logs' AS signal_type{facet}, \
             COUNT(*) AS total, \
             SUM(CASE WHEN SeverityText IN ('ERROR', 'FATAL') THEN 1 ELSE 0 END) AS errors \
             FROM otel_logs t{time_filter}{group_by} \
             UNION ALL \
             SELECT 'metrics' AS signal_type{facet}, \
             COUNT(DISTINCT MetricName) AS total, \
             0 AS errors \
             FROM otel_metrics t{time_filter}{group_by}"
        ))
    }

    fn compile_anomaly(
        &self,
        field: &str,
        _compared_to: &Option<Baseline>,
        cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let table = "otel_traces";
        let col = self.resolve_column(field, table)?;

        let mut where_parts = Vec::new();
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, table)?);
        }
        if let Some(ref cond) = cq.conditions {
            where_parts.push(self.compile_condition(cond, table)?);
        }
        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };

        let facet_select = if let Some(ref f) = cq.facet {
            format!(", {}", f.dimension)
        } else {
            String::new()
        };

        let facet_partition = if let Some(ref f) = cq.facet {
            format!(" PARTITION BY {}", f.dimension)
        } else {
            String::new()
        };

        Ok(format!(
            "SELECT *{facet_select}, \
             ({col} - AVG({col}) OVER({facet_partition})) / \
             NULLIF(STDDEV({col}) OVER({facet_partition}), 0) AS z_score \
             FROM {table}{where_clause} \
             ORDER BY z_score DESC"
        ))
    }

    fn compile_path_deviation(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        if !self.capabilities.topic_data {
            return Err(ROSQLError::DataSourceUnavailable {
                data_source: "topic_messages".into(),
                message: "PATH DEVIATION requires '/odom' topic data. \
                         Configure topic ingest for this data source."
                    .into(),
            });
        }

        let mut where_parts = vec!["topic_name = '/odom'".to_string()];
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "topic_messages")?);
        }
        if let Some(RobotScope::Single(ref id)) = cq.robot_scope {
            where_parts.push(format!("robot_id = '{id}'"));
        }

        Ok(format!(
            "SELECT robot_id, timestamp, \
             fields->>'position.x' AS x, \
             fields->>'position.y' AS y, \
             fields->>'orientation.z' AS theta \
             FROM topic_messages \
             WHERE {} \
             ORDER BY timestamp",
            where_parts.join(" AND ")
        ))
    }

    fn compile_show_recording(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        if !self.capabilities.recording_index {
            return Err(ROSQLError::DataSourceUnavailable {
                data_source: "mcap_metadata".into(),
                message: "SHOW RECORDING requires an mcap_metadata table. \
                         Configure your recording index to enable this feature."
                    .into(),
            });
        }

        let mut where_parts = Vec::new();
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "mcap_metadata")?);
        }
        if let Some(RobotScope::Single(ref id)) = cq.robot_scope {
            where_parts.push(format!("robot_id = '{id}'"));
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };

        Ok(format!(
            "SELECT robot_id, session_id, start_time, end_time, s3_key, topics \
             FROM mcap_metadata{where_clause} \
             ORDER BY start_time DESC"
        ))
    }

    fn compile_correlate(
        &self,
        with_source: &DataSource,
        cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let table_b = self.resolve_table(with_source)?;
        let mut where_parts = Vec::new();
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "a")?);
        }
        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };

        let facet_group = if let Some(ref f) = cq.facet {
            format!(" GROUP BY {}", f.dimension)
        } else {
            String::new()
        };

        Ok(format!(
            "SELECT {corr} AS correlation{facet_group} \
             FROM otel_traces a \
             JOIN {table_b} b ON a.Timestamp = b.Timestamp{where_clause}",
            corr = self.dialect.corr_aggregate("a.Duration", "b.Value"),
        ))
    }

    fn compile_during(
        &self,
        inner_source: &DataSource,
        inner_conditions: &Option<Condition>,
        inner_time_range: &Option<TimeRange>,
        _cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let inner_table = self.resolve_table(inner_source)?;

        let mut inner_where = Vec::new();
        if let Some(ref cond) = inner_conditions {
            inner_where.push(self.compile_condition(cond, &inner_table)?);
        }
        if let Some(ref tr) = inner_time_range {
            inner_where.push(self.compile_time_range(tr, &inner_table)?);
        }

        let inner_where_clause = if inner_where.is_empty() {
            String::new()
        } else {
            format!(" AND {}", inner_where.join(" AND "))
        };

        Ok(format!(
            "SELECT outer_t.* FROM otel_traces outer_t \
             WHERE EXISTS (\
             SELECT 1 FROM {inner_table} inner_t \
             WHERE inner_t.Timestamp >= outer_t.Timestamp \
             AND inner_t.Timestamp <= outer_t.Timestamp{inner_where_clause}\
             )"
        ))
    }

    // ── Helpers ─────────────────────────────────────────────────────

    fn compile_compound_suffix(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let mut parts = Vec::new();

        let mut where_parts = Vec::new();
        if let Some(ref tr) = cq.time_range {
            // Use a generic table ref since compound queries may span CTEs
            where_parts.push(self.compile_time_range(tr, "")?);
        }
        if let Some(ref cond) = cq.conditions {
            where_parts.push(self.compile_condition(cond, "")?);
        }

        if !where_parts.is_empty() {
            // The CTE already has a terminal SELECT; append WHERE to it
            // Actually, we need to restructure: wrap the CTE result
        }

        if let Some(ref ob) = cq.order_by {
            let dir = match ob.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            parts.push(format!(" ORDER BY {} {dir}", ob.field));
        }

        if let Some(limit) = cq.limit {
            parts.push(format!(" LIMIT {limit}"));
        }

        Ok(parts.join(""))
    }

    fn compile_selections(&self, sels: &[Selection], table: &str) -> Result<String, ROSQLError> {
        let mut cols = Vec::new();
        for sel in sels {
            cols.push(self.compile_selection(sel, table)?);
        }
        Ok(cols.join(", "))
    }

    fn compile_selection(&self, sel: &Selection, table: &str) -> Result<String, ROSQLError> {
        match sel {
            Selection::Star => Ok("*".into()),
            Selection::Field(name) => self.resolve_column(name, table),
            Selection::Aggregation(agg) => self.compile_aggregation(agg, table),
            Selection::Aliased { expr, alias } => {
                let inner = self.compile_selection(expr, table)?;
                Ok(format!("{inner} AS {alias}"))
            }
        }
    }

    fn compile_aggregation(
        &self,
        agg: &AggregationCall,
        table: &str,
    ) -> Result<String, ROSQLError> {
        let fn_name = match agg.function {
            AggregationFn::Count => "COUNT",
            AggregationFn::Sum => "SUM",
            AggregationFn::Avg => "AVG",
            AggregationFn::Min => "MIN",
            AggregationFn::Max => "MAX",
            AggregationFn::Stddev => "STDDEV",
            AggregationFn::Percentile => {
                // Special handling: PERCENTILE(field, pct)
                if agg.args.len() == 2 {
                    let col = self.compile_expr(&agg.args[0], table)?;
                    let _pct = self.compile_expr(&agg.args[1], table)?;
                    let fraction = match &agg.args[1] {
                        Expr::Literal(Literal::Integer(n)) => *n as f64 / 100.0,
                        Expr::Literal(Literal::Float(f)) => *f / 100.0,
                        _ => 0.5,
                    };
                    return Ok(self.dialect.percentile_cont(fraction, &col));
                }
                "PERCENTILE_CONT"
            }
            AggregationFn::Rate => "RATE",
            AggregationFn::Delta => "DELTA",
            AggregationFn::Derivative => "DERIVATIVE",
            AggregationFn::MovingAvg => "MOVING_AVG",
            AggregationFn::TopicRate => "TOPIC_RATE",
            AggregationFn::NodeStatus => "NODE_STATUS",
            AggregationFn::Expected => "EXPECTED",
            AggregationFn::ActionSuccessRate => "ACTION_SUCCESS_RATE",
            AggregationFn::Uptime => "UPTIME",
            AggregationFn::ApproxCountDistinct => "APPROX_COUNT_DISTINCT",
            AggregationFn::ApproxPercentile => "APPROX_PERCENTILE",
        };

        let args: Result<Vec<_>, _> = agg
            .args
            .iter()
            .map(|a| self.compile_expr(a, table))
            .collect();
        let args = args?;

        Ok(format!("{}({})", fn_name, args.join(", ")))
    }

    fn compile_from(&self, _source: &DataSource, table: &str) -> Result<String, ROSQLError> {
        Ok(format!("FROM {table}"))
    }

    fn compile_condition(&self, cond: &Condition, table: &str) -> Result<String, ROSQLError> {
        match cond {
            Condition::Comparison { left, op, right } => {
                let l = self.compile_expr(left, table)?;
                let r = self.compile_expr_with_field_context(right, left, table)?;
                let op_str = match op {
                    ComparisonOp::Eq => "=",
                    ComparisonOp::Neq => "!=",
                    ComparisonOp::Lt => "<",
                    ComparisonOp::Gt => ">",
                    ComparisonOp::Lte => "<=",
                    ComparisonOp::Gte => ">=",
                };
                Ok(format!("{l} {op_str} {r}"))
            }
            Condition::And(a, b) => {
                let la = self.compile_condition(a, table)?;
                let lb = self.compile_condition(b, table)?;
                Ok(format!("({la} AND {lb})"))
            }
            Condition::Or(a, b) => {
                let la = self.compile_condition(a, table)?;
                let lb = self.compile_condition(b, table)?;
                Ok(format!("({la} OR {lb})"))
            }
            Condition::Not(inner) => {
                let s = self.compile_condition(inner, table)?;
                Ok(format!("NOT ({s})"))
            }
            Condition::IsNull(expr) => {
                let e = self.compile_expr(expr, table)?;
                Ok(format!("{e} IS NULL"))
            }
            Condition::IsNotNull(expr) => {
                let e = self.compile_expr(expr, table)?;
                Ok(format!("{e} IS NOT NULL"))
            }
            Condition::In { expr, values } => {
                let e = self.compile_expr(expr, table)?;
                let vals: Result<Vec<_>, _> =
                    values.iter().map(|v| self.compile_expr(v, table)).collect();
                Ok(format!("{e} IN ({})", vals?.join(", ")))
            }
            Condition::Like { expr, pattern } => {
                let e = self.compile_expr(expr, table)?;
                Ok(format!("{e} LIKE '{pattern}'"))
            }
            Condition::Between { expr, low, high } => {
                let e = self.compile_expr(expr, table)?;
                let l = self.compile_expr(low, table)?;
                let h = self.compile_expr(high, table)?;
                Ok(format!("{e} BETWEEN {l} AND {h}"))
            }
        }
    }

    fn compile_expr(&self, expr: &Expr, table: &str) -> Result<String, ROSQLError> {
        match expr {
            Expr::Field(name) => self.resolve_column(name, table),
            Expr::Literal(lit) => Ok(compile_literal(lit)),
            Expr::UnitValue(uv) => {
                // Default: use SI value
                Ok(format!("{}", uv.si_value))
            }
            Expr::Aggregation(agg) => self.compile_aggregation(agg, table),
            Expr::FieldAccess { base, key } => Ok(self.dialect.json_access(base, key)),
            Expr::BinaryOp { left, op, right } => {
                let l = self.compile_expr(left, table)?;
                let r = self.compile_expr(right, table)?;
                let op_str = match op {
                    ArithmeticOp::Add => "+",
                    ArithmeticOp::Sub => "-",
                    ArithmeticOp::Mul => "*",
                    ArithmeticOp::Div => "/",
                };
                Ok(format!("({l} {op_str} {r})"))
            }
        }
    }

    /// Compile an expression, converting UnitValue to the storage unit of a
    /// context field (the left-hand side of a comparison).
    fn compile_expr_with_field_context(
        &self,
        expr: &Expr,
        context_field: &Expr,
        table: &str,
    ) -> Result<String, ROSQLError> {
        if let Expr::UnitValue(uv) = expr {
            // Try to find the storage unit of the context field
            if let Expr::Field(field_name) = context_field {
                if let Some(field_def) = self.registry.resolve(field_name) {
                    if let Some(ref storage_unit) = field_def.storage_unit {
                        let converted =
                            convert_si_to_storage(uv.si_value, &uv.si_unit, storage_unit);
                        return Ok(format!("{converted}"));
                    }
                }
            }
            // Fallback: use SI value
            return Ok(format!("{}", uv.si_value));
        }
        self.compile_expr(expr, table)
    }

    fn compile_time_range(&self, tr: &TimeRange, _table: &str) -> Result<String, ROSQLError> {
        let ts_col = self.dialect.timestamp_column();
        match tr {
            TimeRange::Since(expr) => {
                let time_val = self.compile_time_expr(expr)?;
                Ok(format!("{ts_col} >= {time_val}"))
            }
            TimeRange::Between { start, end } => {
                let s = self.compile_time_expr(start)?;
                let e = self.compile_time_expr(end)?;
                Ok(format!("{ts_col} >= {s} AND {ts_col} <= {e}"))
            }
        }
    }

    fn compile_time_expr(&self, expr: &TimeExpr) -> Result<String, ROSQLError> {
        match expr {
            TimeExpr::Relative(rt) => Ok(self.dialect.interval_ago(rt.amount, &rt.unit)),
            TimeExpr::Absolute(ts) => Ok(format!("'{ts}'")),
            TimeExpr::Epoch(epoch) => match epoch {
                UnixEpoch::Seconds(v) => Ok(self.dialect.from_epoch_seconds(*v)),
                UnixEpoch::Milliseconds(v) => Ok(self.dialect.from_epoch_seconds(*v / 1000)),
                UnixEpoch::Nanoseconds(v) => {
                    Ok(self.dialect.from_epoch_seconds(*v / 1_000_000_000))
                }
            },
            TimeExpr::Anchor(anchor) => {
                // Lifecycle anchors require platform-specific resolution.
                // For the open source driver, only LastActionFailure is supported
                // (from otel_traces).
                match anchor {
                    LifecycleAnchor::LastActionFailure => Ok(format!(
                        "(SELECT MAX(Timestamp) FROM otel_traces \
                         WHERE StatusCode = 'ERROR' AND {} IS NOT NULL)",
                        self.dialect
                            .json_access("SpanAttributes", "ros.action.name")
                    )),
                    _ => Err(ROSQLError::DataSourceUnavailable {
                        data_source: "robot_heartbeats".into(),
                        message: format!(
                            "Lifecycle anchor '{anchor:?}' requires robot_heartbeats data \
                             (Robot Ops platform). Only 'last action failure' is available \
                             in the open source driver."
                        ),
                    }),
                }
            }
        }
    }

    fn resolve_table(&self, source: &DataSource) -> Result<String, ROSQLError> {
        // Check capability requirements
        match source {
            DataSource::Topics | DataSource::TopicAlias(_) | DataSource::Tf => {
                if !self.capabilities.topic_data {
                    return Err(ROSQLError::DataSourceUnavailable {
                        data_source: format!("{source:?}"),
                        message: "This data source requires topic ingest. \
                                 Configure topic ingest to enable this feature."
                            .into(),
                    });
                }
            }
            DataSource::Recordings => {
                if !self.capabilities.recording_index {
                    return Err(ROSQLError::DataSourceUnavailable {
                        data_source: "recordings".into(),
                        message: "FROM recordings requires an mcap_metadata table. \
                                 Configure your recording index to enable this feature."
                            .into(),
                    });
                }
            }
            _ => {}
        }

        self.registry
            .table_name(source)
            .map(|s| s.to_string())
            .ok_or_else(|| ROSQLError::CompilationError {
                message: format!("no table mapping for data source {:?}", source),
            })
    }

    fn resolve_column(&self, field_name: &str, _table: &str) -> Result<String, ROSQLError> {
        // Check if it's a wildcard field like "span_attrs.foo"
        if field_name == "*" {
            return Ok("*".into());
        }

        if let Some(field_def) = self.registry.resolve(field_name) {
            if field_def.is_map_access {
                if let (Some(ref map_col), Some(ref map_key)) =
                    (&field_def.map_column, &field_def.map_key)
                {
                    return Ok(self.dialect.json_access(map_col, map_key));
                }
            }
            return Ok(field_def.column.clone());
        }

        // Unknown field — pass through as-is (could be a raw column name)
        Ok(field_name.to_string())
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

fn compile_literal(lit: &Literal) -> String {
    match lit {
        Literal::Integer(n) => n.to_string(),
        Literal::Float(f) => f.to_string(),
        Literal::String(s) => format!("'{s}'"),
        Literal::Boolean(b) => {
            if *b {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        Literal::Null => "NULL".into(),
    }
}

/// Convert a value from SI to a storage unit.
/// E.g., 0.5 s → 500000000 ns.
fn convert_si_to_storage(si_value: f64, si_unit: &str, storage_unit: &str) -> f64 {
    // If units match, no conversion needed
    if si_unit == storage_unit {
        return si_value;
    }

    // Time conversions: SI is seconds
    if si_unit == "s" {
        match storage_unit {
            "ns" => return si_value * 1e9,
            "us" => return si_value * 1e6,
            "ms" => return si_value * 1e3,
            "s" => return si_value,
            _ => {}
        }
    }

    // Distance: SI is meters
    if si_unit == "m" {
        match storage_unit {
            "mm" => return si_value * 1e3,
            "cm" => return si_value * 1e2,
            "km" => return si_value / 1e3,
            _ => {}
        }
    }

    // Fallback: return SI value unchanged
    si_value
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::otel_registry::default_otel_registry;

    fn pg() -> SqlDialect {
        SqlDialect::PostgreSQL
    }

    fn caps() -> BackendCapabilities {
        BackendCapabilities {
            topic_data: true,
            recording_index: true,
        }
    }

    fn compile_pg(query: &str) -> String {
        let ast = crate::parse(query).unwrap();
        let reg = default_otel_registry();
        compile(&ast, &reg, &pg(), &caps()).unwrap()
    }

    fn compile_pg_err(query: &str) -> ROSQLError {
        let ast = crate::parse(query).unwrap();
        let reg = default_otel_registry();
        compile(&ast, &reg, &pg(), &caps()).unwrap_err()
    }

    #[test]
    fn basic_select_star() {
        let sql = compile_pg("FROM logs");
        assert_eq!(sql, "SELECT * FROM otel_logs");
    }

    #[test]
    fn select_fields() {
        let sql = compile_pg("SELECT span_name, duration FROM traces");
        assert_eq!(sql, "SELECT SpanName, Duration FROM otel_traces");
    }

    #[test]
    fn where_comparison() {
        let sql = compile_pg("FROM traces WHERE status = 'ERROR'");
        assert!(sql.contains("WHERE StatusCode = 'ERROR'"));
    }

    #[test]
    fn where_unit_value_converts_to_storage() {
        let sql = compile_pg("FROM traces WHERE duration > 500 ms");
        // 500ms = 0.5s → 500000000 ns (storage unit is ns)
        assert!(sql.contains("500000000"), "got: {sql}");
    }

    #[test]
    fn since_relative() {
        let sql = compile_pg("FROM logs SINCE 30 minutes ago");
        assert!(
            sql.contains("Timestamp >= NOW() - INTERVAL '30 minute'"),
            "got: {sql}"
        );
    }

    #[test]
    fn since_absolute() {
        let sql = compile_pg("FROM logs SINCE '2026-03-18T14:00:00Z'");
        assert!(sql.contains("Timestamp >= '2026-03-18T14:00:00Z'"));
    }

    #[test]
    fn since_unix_epoch() {
        let sql = compile_pg("FROM logs SINCE 1742306400");
        assert!(sql.contains("to_timestamp(1742306400)"));
    }

    #[test]
    fn facet_group_by() {
        let sql = compile_pg("FROM logs FACET robot_id");
        assert!(sql.contains("GROUP BY robot_id"));
    }

    #[test]
    fn order_by_desc() {
        let sql = compile_pg("FROM traces ORDER BY duration DESC");
        assert!(sql.contains("ORDER BY Duration DESC"));
    }

    #[test]
    fn limit_clause() {
        let sql = compile_pg("FROM logs LIMIT 10");
        assert!(sql.contains("LIMIT 10"));
    }

    #[test]
    fn map_field_access() {
        let sql = compile_pg("FROM traces WHERE node = '/planner'");
        assert!(sql.contains("SpanAttributes->>'ros.node'"), "got: {sql}");
    }

    #[test]
    fn bracket_field_access() {
        let sql = compile_pg("FROM logs WHERE fields['my_key'] = 'val'");
        assert!(sql.contains("fields->>'my_key'"), "got: {sql}");
    }

    #[test]
    fn aggregation_avg() {
        let sql = compile_pg("SELECT AVG(duration) FROM traces");
        assert!(sql.contains("AVG(Duration)"));
    }

    #[test]
    fn aggregation_count_star() {
        let sql = compile_pg("SELECT COUNT(*) FROM logs");
        assert!(sql.contains("COUNT(*)"));
    }

    #[test]
    fn topic_alias_odom() {
        let sql = compile_pg("FROM odom SINCE 10 minutes ago");
        assert!(sql.contains("FROM topic_messages"));
        assert!(sql.contains("topic_name = '/odom'"));
    }

    #[test]
    fn pipeline_compiles() {
        let sql = compile_pg("FROM traces | WHERE duration > 500 ms | FACET robot_id");
        assert!(sql.contains("FROM otel_traces"));
        assert!(sql.contains("GROUP BY robot_id"));
    }

    // ── Compound clauses ────────────────────────────────────────────

    #[test]
    fn message_journey() {
        let sql = compile_pg("MESSAGE JOURNEY FOR TRACE 'abc123'");
        assert!(sql.contains("WITH RECURSIVE journey"));
        assert!(sql.contains("TraceId = 'abc123'"));
        assert!(sql.contains("ParentSpanId"));
    }

    #[test]
    fn trace_query() {
        let sql = compile_pg("TRACE 'abc123'");
        assert!(sql.contains("TraceId = 'abc123'"));
    }

    #[test]
    fn health_query() {
        let sql = compile_pg("HEALTH() SINCE 30 minutes ago");
        assert!(sql.contains("signal_type"));
        assert!(sql.contains("UNION ALL"));
        assert!(sql.contains("otel_traces"));
        assert!(sql.contains("otel_logs"));
        assert!(sql.contains("otel_metrics"));
    }

    #[test]
    fn anomaly_query() {
        let sql = compile_pg("ANOMALY(duration) SINCE 24 hours ago");
        assert!(sql.contains("z_score"), "got: {sql}");
        assert!(sql.contains("STDDEV"));
        assert!(sql.contains("AVG"));
    }

    #[test]
    fn show_recording() {
        let sql = compile_pg("SHOW RECORDING SINCE yesterday");
        assert!(sql.contains("mcap_metadata"));
        assert!(sql.contains("s3_key"));
    }

    #[test]
    fn path_deviation_no_capability() {
        let ast = crate::parse("PATH DEVIATION FOR ROBOT 'r1' SINCE yesterday").unwrap();
        let reg = default_otel_registry();
        let no_topics = BackendCapabilities {
            topic_data: false,
            recording_index: false,
        };
        let err = compile(&ast, &reg, &pg(), &no_topics).unwrap_err();
        assert!(matches!(err, ROSQLError::DataSourceUnavailable { .. }));
    }

    #[test]
    fn show_recording_no_capability() {
        let ast = crate::parse("SHOW RECORDING SINCE yesterday").unwrap();
        let reg = default_otel_registry();
        let no_recordings = BackendCapabilities {
            topic_data: false,
            recording_index: false,
        };
        let err = compile(&ast, &reg, &pg(), &no_recordings).unwrap_err();
        assert!(matches!(err, ROSQLError::DataSourceUnavailable { .. }));
    }

    #[test]
    fn correlate_query() {
        let sql = compile_pg("CORRELATE WITH metrics SINCE 7 days ago");
        assert!(sql.contains("CORR("), "got: {sql}");
    }

    #[test]
    fn since_last_action_failure() {
        let sql = compile_pg("FROM traces SINCE last action failure");
        assert!(sql.contains("MAX(Timestamp)"), "got: {sql}");
        assert!(sql.contains("StatusCode = 'ERROR'"), "got: {sql}");
    }

    #[test]
    fn since_last_deployment_unavailable() {
        let err = compile_pg_err("FROM traces SINCE last deployment");
        assert!(matches!(err, ROSQLError::DataSourceUnavailable { .. }));
    }
}
