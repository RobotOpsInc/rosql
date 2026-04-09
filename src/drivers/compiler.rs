//! AST → SQL compilation.
//!
//! Pure functions that transform a parsed ROSQL AST into a SQL string.
//! No database connection is needed — this module is fully testable in isolation.

use crate::ast::*;
use crate::error::ROSQLError;

use super::dialect::SqlDialect;
use super::field_registry::FieldRegistry;
use super::BackendCapabilities;

/// Result of compiling a ROSQL AST to SQL.
#[derive(Debug)]
pub struct CompileResult {
    /// The compiled SQL string.
    pub sql: String,
    /// Whether a default LIMIT was injected (no explicit LIMIT in the query).
    pub default_limit_applied: bool,
}

/// Compile a ROSQL AST to SQL, optionally injecting a default LIMIT.
///
/// Pass `default_limit: Some(100)` to automatically cap result sets at 100 rows
/// for queries that don't have an explicit LIMIT. Certain query shapes are exempt
/// (scalar aggregations, FACET queries, TRACE, MESSAGE JOURNEY/PATHS/PATH).
pub fn compile(
    query: &Query,
    registry: &FieldRegistry,
    dialect: &SqlDialect,
    capabilities: &BackendCapabilities,
    default_limit: Option<u64>,
) -> Result<CompileResult, ROSQLError> {
    let ctx = CompileCtx {
        registry,
        dialect,
        capabilities,
    };

    // Apply default LIMIT when none is present and the query type is not exempt.
    let (query_with_limit, default_limit_applied) = apply_default_limit(query, default_limit);
    let query_ref = query_with_limit.as_ref().unwrap_or(query);

    let sql = match query_ref {
        Query::Standard(sq) => ctx.compile_standard(sq),
        Query::Pipeline(pq) => ctx.compile_pipeline(pq),
        Query::Compound(cq) => ctx.compile_compound(cq),
    }?;

    Ok(CompileResult {
        sql,
        default_limit_applied,
    })
}

/// Determine whether a query is exempt from the default LIMIT.
fn is_limit_exempt(query: &Query) -> bool {
    match query {
        Query::Standard(sq) => {
            // Exempt if all selections are aggregations and no FACET (pure scalar agg)
            let all_agg = !sq.selections.is_empty()
                && sq.selections.iter().all(|s| match s {
                    Selection::Aggregation(_) => true,
                    Selection::Aliased { expr, .. } => {
                        matches!(expr.as_ref(), Selection::Aggregation(_))
                    }
                    _ => false,
                });
            all_agg || sq.facet.is_some()
        }
        Query::Pipeline(pq) => {
            // Check normalized: if it has a FACET stage or only agg SELECT, exempt.
            let has_facet = pq
                .stages
                .iter()
                .any(|s| matches!(s, PipelineStage::Facet(_)));
            let all_agg = pq.stages.iter().any(|s| {
                if let PipelineStage::Select(sels) = s {
                    !sels.is_empty()
                        && sels
                            .iter()
                            .all(|sel| matches!(sel, Selection::Aggregation(_)))
                } else {
                    false
                }
            });
            has_facet || all_agg
        }
        Query::Compound(cq) => matches!(
            cq.clause,
            CompoundClause::Trace { .. }
                | CompoundClause::MessageJourney { .. }
                | CompoundClause::MessagePaths { .. }
                | CompoundClause::MessagePath { .. }
        ),
    }
}

/// Returns a modified query with a default LIMIT injected (if applicable) and a flag
/// indicating whether the limit was applied. Returns `None` for the query if unchanged.
fn apply_default_limit(query: &Query, default_limit: Option<u64>) -> (Option<Query>, bool) {
    let limit = match default_limit {
        Some(n) => n,
        None => return (None, false),
    };

    if is_limit_exempt(query) {
        return (None, false);
    }

    match query {
        Query::Standard(sq) if sq.limit.is_none() => {
            let mut sq2 = sq.clone();
            sq2.limit = Some(limit);
            (Some(Query::Standard(sq2)), true)
        }
        Query::Pipeline(pq) => {
            // Inject if no Limit stage present
            if pq
                .stages
                .iter()
                .any(|s| matches!(s, PipelineStage::Limit(_)))
            {
                (None, false)
            } else {
                let mut pq2 = pq.clone();
                pq2.stages.push(PipelineStage::Limit(limit));
                (Some(Query::Pipeline(pq2)), true)
            }
        }
        Query::Compound(cq) if cq.limit.is_none() => {
            let mut cq2 = cq.clone();
            cq2.limit = Some(limit);
            (Some(Query::Compound(cq2)), true)
        }
        _ => (None, false),
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
    /// Resolve a well-known ROSQL field to its quoted column name.
    /// Falls back to the field name itself if not found in the registry.
    fn col(&self, field_name: &str) -> String {
        self.resolve_column(field_name, "")
            .unwrap_or_else(|_| field_name.to_string())
    }

    /// Quote a table name for the current dialect.
    fn qtable(&self, table: &str) -> String {
        self.dialect.quote_ident(table)
    }

    // ── Standard query ──────────────────────────────────────────────

    fn compile_standard(&self, q: &ROSQLQuery) -> Result<String, ROSQLError> {
        let table = self.resolve_table(&q.data_source)?;
        let mut parts = Vec::new();

        // SELECT — when FACET is present and no explicit columns chosen, emit
        // "{facet_col}, COUNT(*) AS count" instead of the invalid "SELECT * … GROUP BY col"
        let select_clause = if let Some(ref facet) = q.facet {
            let is_star = matches!(q.selections.as_slice(), [crate::ast::Selection::Star]);
            if is_star {
                let col = self.resolve_column(&facet.dimension, &table)?;
                format!("{col}, COUNT(*) AS count")
            } else {
                self.compile_selections(&q.selections, &table)?
            }
        } else {
            self.compile_selections(&q.selections, &table)?
        };
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

        // LIMIT / OFFSET
        if let Some(limit) = q.limit {
            parts.push(format!("LIMIT {limit}"));
        }
        if let Some(offset) = q.offset {
            parts.push(format!("OFFSET {offset}"));
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
            offset: None,
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
                PipelineStage::Offset(n) => sq.offset = Some(*n),
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
            CompoundClause::ShowTraceBreakdown => Err(ROSQLError::NotImplemented {
                feature: "SHOW TRACE_BREAKDOWN".into(),
                message: "SHOW TRACE_BREAKDOWN is being replaced by SHOW SPAN SUMMARY. \
                           Use `SELECT span_name, COUNT(*) AS count, AVG(duration) AS avg_duration \
                           FROM traces GROUP BY span_name` as a workaround."
                    .into(),
            }),
            CompoundClause::Health => Err(ROSQLError::NotImplemented {
                feature: "HEALTH()".into(),
                message: "HEALTH() is being redesigned. Run these queries separately: \
                           error rate (SELECT COUNT(*) FROM traces WHERE status_code='ERROR'), \
                           log severity (SELECT severity, COUNT(*) FROM logs GROUP BY severity), \
                           and metric counts (SELECT COUNT(*) FROM metrics). \
                           See the ROSQL cookbook for a complete health dashboard recipe."
                    .into(),
            }),
            CompoundClause::Anomaly { .. } => Err(ROSQLError::NotImplemented {
                feature: "ANOMALY()".into(),
                message: "ANOMALY() is being redesigned. Use manual z-score computation: \
                           SELECT *, (value - AVG(value) OVER()) / NULLIF(STDDEV(value) OVER(), 0) \
                           AS z_score FROM metrics."
                    .into(),
            }),
            CompoundClause::PathDeviation { .. } => Err(ROSQLError::NotImplemented {
                feature: "PATH DEVIATION".into(),
                message: "PATH DEVIATION requires redesign. \
                           Use `SELECT * FROM odom` to retrieve raw odometry data."
                    .into(),
            }),
            CompoundClause::Correlate { .. } => Err(ROSQLError::NotImplemented {
                feature: "CORRELATE WITH".into(),
                message: "CORRELATE WITH requires redesign. \
                           Use a manual JOIN between the two data sources with a time-window condition."
                    .into(),
            }),
            CompoundClause::ShowRecording => Err(ROSQLError::NotImplemented {
                feature: "SHOW RECORDING".into(),
                message: "SHOW RECORDING is being replaced by improved `FROM recordings` syntax."
                    .into(),
            }),
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
        let tbl = self.qtable("otel_traces");
        let tid = self.col("trace_id");
        let psid = self.col("parent_span_id");
        let sid = self.col("span_id");
        let mut sql = format!(
            "WITH RECURSIVE journey AS (\
             SELECT * FROM {tbl} WHERE {tid} = '{trace_id}' AND {psid} = '' \
             UNION ALL \
             SELECT t.* FROM {tbl} t \
             JOIN journey j ON t.{psid} = j.{sid}\
             ) SELECT * FROM journey"
        );
        sql.push_str(&self.compile_compound_suffix(cq)?);
        Ok(sql)
    }

    fn compile_message_paths(&self, topic: &str, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let psid = self.col("parent_span_id");
        let sid = self.col("span_id");
        let topic_attr = self.dialect.json_access(
            self.registry
                .resolve("topic")
                .map(|f| f.column.as_str())
                .unwrap_or("span_attributes"),
            "ros.topic",
        );
        let mut sql = format!(
            "WITH RECURSIVE paths AS (\
             SELECT * FROM {tbl} WHERE {topic_attr} = '{topic}' \
             UNION ALL \
             SELECT t.* FROM {tbl} t \
             JOIN paths p ON t.{psid} = p.{sid}\
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
        let tbl = self.qtable("otel_traces");
        let psid = self.col("parent_span_id");
        let sid = self.col("span_id");
        let span_attrs_col = self
            .registry
            .resolve("topic")
            .map(|f| f.column.as_str())
            .unwrap_or("span_attributes");
        let topic_attr = self.dialect.json_access(span_attrs_col, "ros.topic");
        let node_attr = self.dialect.json_access(span_attrs_col, "ros.node");
        let mut sql = format!(
            "WITH RECURSIVE msg_path AS (\
             SELECT * FROM {tbl} WHERE {topic_attr} = '{from_topic}' \
             UNION ALL \
             SELECT t.* FROM {tbl} t \
             JOIN msg_path p ON t.{psid} = p.{sid}\
             ) SELECT * FROM msg_path WHERE {node_attr} = '{to_node}'"
        );
        sql.push_str(&self.compile_compound_suffix(cq)?);
        Ok(sql)
    }

    fn compile_trace(&self, trace_id: &str) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let tid = self.col("trace_id");
        let ts = self.col("timestamp");
        Ok(format!(
            "SELECT * FROM {tbl} WHERE {tid} = '{trace_id}' ORDER BY {ts}"
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

        let traces_tbl = self.qtable("otel_traces");
        let ts = self.col("timestamp");
        Ok(format!(
            "SELECT outer_t.* FROM {traces_tbl} outer_t \
             WHERE EXISTS (\
             SELECT 1 FROM {inner_table} inner_t \
             WHERE inner_t.{ts} >= outer_t.{ts} \
             AND inner_t.{ts} <= outer_t.{ts}{inner_where_clause}\
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
        if let Some(offset) = cq.offset {
            parts.push(format!(" OFFSET {offset}"));
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
        match agg.function {
            // ── Standard SQL aggregations ─────────────────────────────
            AggregationFn::Count => {
                let args = self.compile_agg_args(agg, table)?;
                Ok(format!("COUNT({})", args.join(", ")))
            }
            AggregationFn::Sum => {
                let args = self.compile_agg_args(agg, table)?;
                Ok(format!("SUM({})", args.join(", ")))
            }
            AggregationFn::Avg => {
                let args = self.compile_agg_args(agg, table)?;
                Ok(format!("AVG({})", args.join(", ")))
            }
            AggregationFn::Min => {
                let args = self.compile_agg_args(agg, table)?;
                Ok(format!("MIN({})", args.join(", ")))
            }
            AggregationFn::Max => {
                let args = self.compile_agg_args(agg, table)?;
                Ok(format!("MAX({})", args.join(", ")))
            }
            AggregationFn::Stddev => {
                let args = self.compile_agg_args(agg, table)?;
                Ok(format!("STDDEV({})", args.join(", ")))
            }
            AggregationFn::Percentile => {
                if agg.args.len() == 2 {
                    let col = self.compile_expr(&agg.args[0], table)?;
                    let fraction = match &agg.args[1] {
                        Expr::Literal(Literal::Integer(n)) => *n as f64 / 100.0,
                        Expr::Literal(Literal::Float(f)) => *f / 100.0,
                        _ => 0.5,
                    };
                    Ok(self.dialect.percentile_cont(fraction, &col))
                } else {
                    Ok("PERCENTILE_CONT".into())
                }
            }

            // ── Implemented robotics/time-series aggregations ─────────

            // TOPIC_RATE([topic_name]) → query otel_metrics for ros2.topic.message_rate
            AggregationFn::TopicRate => {
                let metrics_table = self.dialect.quote_ident("otel_metrics");
                let topic_filter = if let Some(arg) = agg.args.first() {
                    let topic = self.compile_expr(arg, table)?;
                    format!(" AND topic_name = {topic}")
                } else {
                    String::new()
                };
                Ok(format!(
                    "(SELECT AVG(metric_value) FROM {metrics_table} \
                     WHERE metric_name = 'ros2.topic.message_rate'{topic_filter})"
                ))
            }

            // ACTION_SUCCESS_RATE([action_name]) → CASE WHEN ratio
            AggregationFn::ActionSuccessRate => {
                if let Some(arg) = agg.args.first() {
                    // With action filter: wrap as subquery
                    let action = self.compile_expr(arg, table)?;
                    let quoted_table = self.dialect.quote_ident(table);
                    Ok(format!(
                        "(SELECT CAST(COUNT(CASE WHEN action_status = 'succeeded' THEN 1 END) \
                         AS DOUBLE PRECISION) / NULLIF(COUNT(*), 0) \
                         FROM {quoted_table} WHERE action_name = {action})"
                    ))
                } else {
                    // Without filter: inline expression
                    Ok(
                        "CAST(COUNT(CASE WHEN action_status = 'succeeded' THEN 1 END) \
                         AS DOUBLE PRECISION) / NULLIF(COUNT(*), 0)"
                            .into(),
                    )
                }
            }

            // MOVING_AVG(field, N) → window function
            AggregationFn::MovingAvg => {
                if agg.args.len() < 2 {
                    return Err(ROSQLError::CompilationError {
                        message: "MOVING_AVG requires two arguments: MOVING_AVG(field, window_size)"
                            .into(),
                    });
                }
                let col = self.compile_expr(&agg.args[0], table)?;
                let window_size = match &agg.args[1] {
                    Expr::Literal(Literal::Integer(n)) => *n,
                    _ => {
                        return Err(ROSQLError::CompilationError {
                            message: "MOVING_AVG window_size must be an integer literal".into(),
                        })
                    }
                };
                let preceding = window_size.saturating_sub(1);
                let ts_col = self.col("timestamp");
                Ok(format!(
                    "AVG({col}) OVER (ORDER BY {ts_col} ROWS BETWEEN {preceding} PRECEDING AND CURRENT ROW)"
                ))
            }

            // DERIVATIVE(field) → LAG-based rate of change per second
            AggregationFn::Derivative => {
                if agg.args.is_empty() {
                    return Err(ROSQLError::CompilationError {
                        message: "DERIVATIVE requires one argument: DERIVATIVE(field)".into(),
                    });
                }
                let col = self.compile_expr(&agg.args[0], table)?;
                let ts_col = self.col("timestamp");
                let lag_val = format!("LAG({col}) OVER (ORDER BY {ts_col})");
                let lag_ts = format!("LAG({ts_col}) OVER (ORDER BY {ts_col})");
                let diff = self.dialect.timestamp_diff_seconds(&ts_col, &lag_ts);
                Ok(format!(
                    "({col} - {lag_val}) / NULLIF({diff}, 0)"
                ))
            }

            // APPROX_COUNT_DISTINCT(field) → dialect-specific
            AggregationFn::ApproxCountDistinct => {
                if agg.args.is_empty() {
                    return Err(ROSQLError::CompilationError {
                        message: "APPROX_COUNT_DISTINCT requires one argument".into(),
                    });
                }
                let col = self.compile_expr(&agg.args[0], table)?;
                Ok(self.dialect.approx_count_distinct(&col))
            }

            // APPROX_PERCENTILE(field, pct) → dialect-specific
            AggregationFn::ApproxPercentile => {
                if agg.args.len() < 2 {
                    return Err(ROSQLError::CompilationError {
                        message: "APPROX_PERCENTILE requires two arguments: APPROX_PERCENTILE(field, percentile)"
                            .into(),
                    });
                }
                let col = self.compile_expr(&agg.args[0], table)?;
                let fraction = match &agg.args[1] {
                    Expr::Literal(Literal::Integer(n)) => *n as f64 / 100.0,
                    Expr::Literal(Literal::Float(f)) => *f / 100.0,
                    _ => 0.5,
                };
                Ok(self.dialect.approx_percentile(fraction, &col))
            }

            // ── Gated features (not yet implemented) ─────────────────

            AggregationFn::NodeStatus => Err(ROSQLError::NotImplemented {
                feature: "NODE_STATUS()".into(),
                message: "NODE_STATUS() requires heartbeat data not yet available in the \
                           open-source schema. Query node health via otel_metrics or otel_logs instead."
                    .into(),
            }),
            AggregationFn::Expected => Err(ROSQLError::NotImplemented {
                feature: "EXPECTED()".into(),
                message: "EXPECTED() requires SLO definitions which are configured in the \
                           Robot Ops platform, not available in open-source ROSQL."
                    .into(),
            }),
            AggregationFn::Uptime => Err(ROSQLError::NotImplemented {
                feature: "UPTIME()".into(),
                message: "UPTIME() requires heartbeat data not yet available in the \
                           open-source schema. Query uptime manually via otel_metrics instead."
                    .into(),
            }),
            AggregationFn::Rate => Err(ROSQLError::NotImplemented {
                feature: "RATE()".into(),
                message: "RATE() is not yet implemented. Use DERIVATIVE(field) for rate of change, \
                           or a manual LAG() window function."
                    .into(),
            }),
            AggregationFn::Delta => Err(ROSQLError::NotImplemented {
                feature: "DELTA()".into(),
                message: "DELTA() is not yet implemented. Use manual subtraction with LAG(): \
                           (field - LAG(field) OVER (ORDER BY timestamp))."
                    .into(),
            }),
        }
    }

    fn compile_agg_args(
        &self,
        agg: &AggregationCall,
        table: &str,
    ) -> Result<Vec<String>, ROSQLError> {
        // Special case: COUNT(*) with no args
        if agg.args.is_empty() {
            return Ok(vec!["*".into()]);
        }
        agg.args
            .iter()
            .map(|a| self.compile_expr(a, table))
            .collect()
    }

    fn compile_from(&self, _source: &DataSource, table: &str) -> Result<String, ROSQLError> {
        let quoted = self.dialect.quote_ident(table);
        Ok(format!("FROM {quoted}"))
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
        let ts_col = self.col("timestamp");
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
                    LifecycleAnchor::LastActionFailure => {
                        let tbl = self.qtable("otel_traces");
                        let ts = self.col("timestamp");
                        let sc = self.col("status");
                        let span_attrs_col = self
                            .registry
                            .resolve("action_name")
                            .map(|f| f.column.as_str())
                            .unwrap_or("span_attributes");
                        let action_attr =
                            self.dialect.json_access(span_attrs_col, "ros.action.name");
                        Ok(format!(
                            "(SELECT MAX({ts}) FROM {tbl} \
                             WHERE {sc} = 'ERROR' AND {action_attr} IS NOT NULL)"
                        ))
                    }
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
            return Ok(self.dialect.quote_ident(&field_def.column));
        }

        // Unknown field — pass through quoted (preserves case for OTel PascalCase columns)
        Ok(self.dialect.quote_ident(field_name))
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
        compile(&ast, &reg, &pg(), &caps(), None).unwrap().sql
    }

    fn compile_pg_err(query: &str) -> ROSQLError {
        let ast = crate::parse(query).unwrap();
        let reg = default_otel_registry();
        compile(&ast, &reg, &pg(), &caps(), None).unwrap_err()
    }

    // Note: PostgreSQL tests use quoted lowercase identifiers (OtelPostgres profile).
    // e.g. "status_code", "otel_traces", "span_attributes"

    #[test]
    fn basic_select_star() {
        let sql = compile_pg("FROM logs");
        assert_eq!(sql, r#"SELECT * FROM "otel_logs""#);
    }

    #[test]
    fn select_fields() {
        let sql = compile_pg("SELECT span_name, duration FROM traces");
        assert!(sql.contains(r#""span_name_col""#), "got: {sql}");
        assert!(sql.contains(r#""duration""#), "got: {sql}");
        assert!(sql.contains(r#""otel_traces""#), "got: {sql}");
    }

    #[test]
    fn where_comparison() {
        let sql = compile_pg("FROM traces WHERE status = 'ERROR'");
        assert!(sql.contains(r#""status_code" = 'ERROR'"#), "got: {sql}");
    }

    #[test]
    fn where_unit_value_converts_to_storage() {
        let sql = compile_pg("FROM traces WHERE duration > 500 ms");
        assert!(sql.contains("500000000"), "got: {sql}");
    }

    #[test]
    fn since_relative() {
        let sql = compile_pg("FROM logs SINCE 30 minutes ago");
        assert!(
            sql.contains(r#""timestamp" >= NOW() - INTERVAL '30 minute'"#),
            "got: {sql}"
        );
    }

    #[test]
    fn since_absolute() {
        let sql = compile_pg("FROM logs SINCE '2026-03-18T14:00:00Z'");
        assert!(
            sql.contains(r#""timestamp" >= '2026-03-18T14:00:00Z'"#),
            "got: {sql}"
        );
    }

    #[test]
    fn since_unix_epoch() {
        let sql = compile_pg("FROM logs SINCE 1742306400");
        assert!(sql.contains("to_timestamp(1742306400)"), "got: {sql}");
    }

    #[test]
    fn facet_group_by() {
        let sql = compile_pg("FROM logs FACET robot_id");
        assert!(sql.contains(r#"GROUP BY "robot_id""#), "got: {sql}");
    }

    #[test]
    fn order_by_desc() {
        let sql = compile_pg("FROM traces ORDER BY duration DESC");
        assert!(sql.contains(r#""duration" DESC"#), "got: {sql}");
    }

    #[test]
    fn limit_clause() {
        let sql = compile_pg("FROM logs LIMIT 10");
        assert!(sql.contains("LIMIT 10"));
    }

    #[test]
    fn map_field_access() {
        let sql = compile_pg("FROM traces WHERE node = '/planner'");
        assert!(
            sql.contains(r#""span_attributes"->>'ros.node'"#),
            "got: {sql}"
        );
    }

    #[test]
    fn bracket_field_access() {
        let sql = compile_pg("FROM logs WHERE fields['my_key'] = 'val'");
        assert!(sql.contains(r#""fields"->>'my_key'"#), "got: {sql}");
    }

    #[test]
    fn aggregation_avg() {
        let sql = compile_pg("SELECT AVG(duration) FROM traces");
        assert!(sql.contains(r#"AVG("duration")"#), "got: {sql}");
    }

    #[test]
    fn aggregation_count_star() {
        let sql = compile_pg("SELECT COUNT(*) FROM logs");
        assert!(sql.contains("COUNT(*)"));
    }

    #[test]
    fn topic_alias_odom() {
        let sql = compile_pg("FROM odom SINCE 10 minutes ago");
        assert!(sql.contains("topic_messages"), "got: {sql}");
        assert!(sql.contains("topic_name = '/odom'"), "got: {sql}");
    }

    #[test]
    fn pipeline_compiles() {
        let sql = compile_pg("FROM traces | WHERE duration > 500 ms | FACET robot_id");
        assert!(sql.contains("otel_traces"), "got: {sql}");
        assert!(sql.contains("GROUP BY"), "got: {sql}");
    }

    // ── Compound clauses ────────────────────────────────────────────

    #[test]
    fn message_journey() {
        let sql = compile_pg("MESSAGE JOURNEY FOR TRACE 'abc123'");
        assert!(sql.contains("WITH RECURSIVE journey"), "got: {sql}");
        assert!(sql.contains("abc123"), "got: {sql}");
        assert!(sql.contains("parent_span_id"), "got: {sql}");
    }

    #[test]
    fn trace_query() {
        let sql = compile_pg("TRACE 'abc123'");
        assert!(sql.contains("abc123"), "got: {sql}");
        assert!(sql.contains("otel_traces"), "got: {sql}");
    }

    #[test]
    fn health_query_gated() {
        let err = compile_pg_err("HEALTH() SINCE 30 minutes ago");
        assert!(
            matches!(err, ROSQLError::NotImplemented { ref feature, .. } if feature == "HEALTH()"),
            "got: {err:?}"
        );
    }

    #[test]
    fn anomaly_query_gated() {
        let err = compile_pg_err("ANOMALY(duration) SINCE 24 hours ago");
        assert!(
            matches!(err, ROSQLError::NotImplemented { ref feature, .. } if feature == "ANOMALY()"),
            "got: {err:?}"
        );
    }

    #[test]
    fn show_recording_gated() {
        let err = compile_pg_err("SHOW RECORDING SINCE yesterday");
        assert!(
            matches!(err, ROSQLError::NotImplemented { ref feature, .. } if feature == "SHOW RECORDING"),
            "got: {err:?}"
        );
    }

    #[test]
    fn path_deviation_gated() {
        let err = compile_pg_err("PATH DEVIATION FOR ROBOT 'r1' SINCE yesterday");
        assert!(
            matches!(err, ROSQLError::NotImplemented { ref feature, .. } if feature == "PATH DEVIATION"),
            "got: {err:?}"
        );
    }

    #[test]
    fn show_recording_no_capability_gated() {
        // SHOW RECORDING is now gated regardless of capabilities
        let err = compile_pg_err("SHOW RECORDING SINCE yesterday");
        assert!(matches!(err, ROSQLError::NotImplemented { .. }));
    }

    #[test]
    fn correlate_query_gated() {
        let err = compile_pg_err("CORRELATE WITH metrics SINCE 7 days ago");
        assert!(
            matches!(err, ROSQLError::NotImplemented { ref feature, .. } if feature == "CORRELATE WITH"),
            "got: {err:?}"
        );
    }

    #[test]
    fn since_last_action_failure() {
        let sql = compile_pg("FROM traces SINCE last action failure");
        assert!(sql.contains("MAX("), "got: {sql}");
        assert!(sql.contains("status_code"), "got: {sql}");
        assert!(sql.contains("ERROR"), "got: {sql}");
    }

    #[test]
    fn since_last_deployment_unavailable() {
        let err = compile_pg_err("FROM traces SINCE last deployment");
        assert!(matches!(err, ROSQLError::DataSourceUnavailable { .. }));
    }
}
