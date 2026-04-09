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
    /// Enrichment plans to execute in phase 2 (empty for non-enriched queries).
    pub enrichments: Vec<EnrichmentPlan>,
    /// Non-fatal compiler warnings (e.g. ANOMALY without FACET).
    pub warnings: Vec<String>,
}

/// A compiled enrichment plan for two-phase execution (ENRICH WITH).
#[derive(Debug, Clone)]
pub struct EnrichmentPlan {
    /// Human-readable data source name (e.g. "logs").
    pub source_name: String,
    /// The enrichment table name (dialect-quoted).
    pub table: String,
    /// The join column used to correlate enrichment rows with primary rows.
    pub join_column: String,
    /// Per-primary-row enrichment row limit.
    pub limit: u64,
    /// If true, skip auto-downsampling for high-frequency topic data.
    pub sample_full: bool,
}

/// Compile a ROSQL AST to SQL, optionally injecting a default LIMIT.
///
/// Pass `default_limit: Some(100)` to automatically cap result sets at 100 rows
/// for queries that don't have an explicit LIMIT. Certain query shapes are exempt
/// (scalar aggregations, FACET queries, TRACE, MESSAGE FLOW, SHOW DEPLOYMENTS, SHOW SPAN SUMMARY).
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

    let (sql, enrichments) = match query_ref {
        Query::Standard(sq) => ctx.compile_standard_with_enrichments(sq),
        Query::Pipeline(pq) => ctx.compile_pipeline_with_enrichments(pq),
        Query::Compound(cq) => ctx.compile_compound(cq).map(|s| (s, vec![])),
    }?;

    // Collect non-fatal warnings (e.g. ANOMALY without FACET).
    let warnings = collect_warnings(query_ref);

    Ok(CompileResult {
        sql,
        default_limit_applied,
        enrichments,
        warnings,
    })
}

/// Collect non-fatal compiler warnings from the (possibly limit-adjusted) query.
fn collect_warnings(query: &Query) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Query::Compound(cq) = query {
        if let CompoundClause::Anomaly { .. } = &cq.clause {
            if cq.facet.is_none() {
                warnings.push(
                    "ANOMALY without FACET compares heterogeneous spans (e.g. heartbeats vs \
                     navigation). Add FACET robot_id or FACET action_name for meaningful z-scores."
                        .into(),
                );
            }
        }
    }
    warnings
}

/// Return a human-readable name for a DataSource.
fn data_source_name(ds: &DataSource) -> String {
    match ds {
        DataSource::Logs => "logs".into(),
        DataSource::SystemLogs => "system_logs".into(),
        DataSource::Traces => "traces".into(),
        DataSource::Metrics => "metrics".into(),
        DataSource::Diagnostics => "diagnostics".into(),
        DataSource::Topics => "topics".into(),
        DataSource::Tf => "tf".into(),
        DataSource::Heartbeats => "heartbeats".into(),
        DataSource::Recordings => "recordings".into(),
        DataSource::Events => "events".into(),
        DataSource::TopicAlias(alias) => alias.topic_name().trim_start_matches('/').into(),
    }
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
            all_agg || sq.facet.is_some() || sq.timeseries.is_some()
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
                | CompoundClause::MessageFlow { .. }
                | CompoundClause::ShowDeployments
                | CompoundClause::ShowSpanSummary
                | CompoundClause::ShowTopics
                | CompoundClause::ShowNodes
                | CompoundClause::ShowNodeGraph
                | CompoundClause::ShowJoints
                | CompoundClause::PathDeviation { .. }
                | CompoundClause::JointDeviation { .. }
                | CompoundClause::Anomaly { .. }
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

    fn compile_standard_with_enrichments(
        &self,
        q: &ROSQLQuery,
    ) -> Result<(String, Vec<EnrichmentPlan>), ROSQLError> {
        let sql = self.compile_standard(q)?;
        let enrichments = self.build_enrichment_plans(&q.data_source, &q.enrichments)?;
        Ok((sql, enrichments))
    }

    fn compile_pipeline_with_enrichments(
        &self,
        pq: &PipelineQuery,
    ) -> Result<(String, Vec<EnrichmentPlan>), ROSQLError> {
        let sq = self.normalize_pipeline(pq)?;
        self.compile_standard_with_enrichments(&sq)
    }

    /// Build enrichment plans from ENRICH WITH clauses.
    fn build_enrichment_plans(
        &self,
        primary_source: &DataSource,
        enrichments: &[EnrichmentClause],
    ) -> Result<Vec<EnrichmentPlan>, ROSQLError> {
        enrichments
            .iter()
            .map(|e| {
                let table = self.resolve_table(&e.source)?;
                let join_column = self.infer_join_key(primary_source, &e.source).to_string();
                let source_name = data_source_name(&e.source);
                Ok(EnrichmentPlan {
                    source_name,
                    table,
                    join_column,
                    limit: e.limit.unwrap_or(50),
                    sample_full: e.sample_full,
                })
            })
            .collect()
    }

    /// Infer the join key column for a primary→enrichment pair.
    fn infer_join_key(&self, primary: &DataSource, enrichment: &DataSource) -> &'static str {
        match (primary, enrichment) {
            // traces → logs: prefer trace_id (OTel logs carry trace_id)
            (DataSource::Traces, DataSource::Logs) => "trace_id",
            // traces → topics/joint_states: trace_id when rmw_robotops is used
            (DataSource::Traces, DataSource::Topics)
            | (DataSource::Traces, DataSource::TopicAlias(_)) => "trace_id",
            // All other combinations: fall back to trace_id as best-effort
            _ => "trace_id",
        }
    }

    fn compile_standard(&self, q: &ROSQLQuery) -> Result<String, ROSQLError> {
        let table = self.resolve_table(&q.data_source)?;
        let mut parts = Vec::new();

        // Resolve the timestamp column for TIMESERIES bucket expressions
        let ts_col = self.col("timestamp");
        let has_timeseries = q.timeseries.is_some();

        // SELECT — when TIMESERIES is present, prepend the time_bucket column.
        // When FACET is present and no explicit columns chosen, emit
        // "{facet_col}, COUNT(*) AS count" instead of the invalid "SELECT * … GROUP BY col"
        let select_clause = if has_timeseries {
            let ts_expr = q
                .timeseries
                .as_ref()
                .map(|ts| {
                    // si_value is in seconds for time units (minutes → 60s, etc.)
                    let seconds = ts.interval.si_value;
                    self.dialect.time_bucket(seconds, &ts_col)
                })
                .unwrap();
            let base = if let Some(ref facet) = q.facet {
                let is_star = matches!(q.selections.as_slice(), [crate::ast::Selection::Star]);
                if is_star {
                    let col = self.resolve_column(&facet.dimension, &table)?;
                    format!("{col}, COUNT(*) AS count")
                } else {
                    self.compile_selections(&q.selections, &table)?
                }
            } else if matches!(q.selections.as_slice(), [crate::ast::Selection::Star]) {
                // Default: COUNT(*) for timeseries without explicit aggregation
                "COUNT(*) AS count".to_string()
            } else {
                self.compile_selections(&q.selections, &table)?
            };
            format!("{ts_expr} AS time_bucket, {base}")
        } else if let Some(ref facet) = q.facet {
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

        // WHERE (conditions + time range + topic alias filter + scope combined)
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
        if let Some(ref scope) = q.scope {
            where_parts.extend(self.compile_scope_filters(scope));
        }
        if !where_parts.is_empty() {
            parts.push(format!("WHERE {}", where_parts.join(" AND ")));
        }

        // GROUP BY — TIMESERIES adds time_bucket; FACET adds its dimension; both can coexist.
        let mut group_by_cols = Vec::new();
        if has_timeseries {
            group_by_cols.push("time_bucket".to_string());
        }
        if let Some(ref facet) = q.facet {
            let col = self.resolve_column(&facet.dimension, &table)?;
            group_by_cols.push(col);
        }
        if !group_by_cols.is_empty() {
            parts.push(format!("GROUP BY {}", group_by_cols.join(", ")));
        }

        // ORDER BY — default to time_bucket ASC for timeseries (unless explicit ORDER BY given)
        if let Some(ref ob) = q.order_by {
            let col = self.resolve_column(&ob.field, &table)?;
            let dir = match ob.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            parts.push(format!("ORDER BY {col} {dir}"));
        } else if has_timeseries {
            parts.push("ORDER BY time_bucket ASC".to_string());
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

    fn normalize_pipeline(&self, pq: &PipelineQuery) -> Result<ROSQLQuery, ROSQLError> {
        let mut sq = ROSQLQuery {
            selections: vec![Selection::Star],
            data_source: DataSource::Logs, // placeholder
            scope: None,
            conditions: None,
            facet: None,
            time_range: None,
            time_basis: None,
            order_by: None,
            limit: None,
            offset: None,
            output_format: None,
            baseline: None,
            timeseries: None,
            enrichments: Vec::new(),
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
                PipelineStage::ForScope(new_scope) => {
                    let existing = sq.scope.get_or_insert_with(QueryScope::empty);
                    if new_scope.robot.is_some() {
                        existing.robot = new_scope.robot.clone();
                    }
                    if new_scope.version.is_some() {
                        existing.version = new_scope.version.clone();
                    }
                    if new_scope.environment.is_some() {
                        existing.environment = new_scope.environment.clone();
                    }
                    if new_scope.session.is_some() {
                        existing.session = new_scope.session.clone();
                    }
                }
                PipelineStage::CompoundClause(_) => {
                    // Compound clauses in pipeline are handled separately
                }
                PipelineStage::Timeseries(ts) => sq.timeseries = Some(ts.clone()),
                PipelineStage::EnrichWith(e) => sq.enrichments.push(e.clone()),
            }
        }

        Ok(sq)
    }

    // ── Compound query ──────────────────────────────────────────────

    fn compile_compound(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        match &cq.clause {
            CompoundClause::Trace { trace_id } => self.compile_trace(trace_id, cq),
            CompoundClause::MessageFlow {
                from_topic,
                to_target,
                ..
            } => self.compile_message_flow(from_topic, to_target, cq),
            CompoundClause::ShowDeployments => self.compile_show_deployments(cq),
            CompoundClause::ShowSpanSummary => self.compile_show_span_summary(cq),
            CompoundClause::ShowPlans { trace_id } => self.compile_show_plans(trace_id, cq),
            CompoundClause::ShowTraceBreakdown => Err(ROSQLError::NotImplemented {
                feature: "SHOW TRACE_BREAKDOWN".into(),
                message: "SHOW TRACE_BREAKDOWN is replaced by SHOW SPAN SUMMARY. \
                           Use `SHOW SPAN SUMMARY` or `SELECT span_name, COUNT(*) AS count, \
                           AVG(duration) AS avg_duration FROM traces FACET span_name` as a workaround."
                    .into(),
            }),
            CompoundClause::Health => Err(ROSQLError::NotImplemented {
                feature: "HEALTH()".into(),
                message: "HEALTH() is being redesigned. Run these queries separately: \
                           error rate (SELECT COUNT(*) FROM traces WHERE status_code='ERROR'), \
                           log severity (SELECT severity, COUNT(*) FROM logs FACET severity), \
                           and metric counts (SELECT COUNT(*) FROM metrics). \
                           See the ROSQL cookbook for a complete health dashboard recipe."
                    .into(),
            }),
            CompoundClause::Anomaly {
                field,
                compared_to,
                data_source,
            } => self.compile_anomaly(field, compared_to, data_source.as_ref(), cq),
            CompoundClause::PathDeviation { target, plan_index } => {
                self.compile_path_deviation(target, *plan_index, cq)
            }
            CompoundClause::JointDeviation { target } => {
                self.compile_joint_deviation(target, cq)
            }
            CompoundClause::Correlate { .. } => Err(ROSQLError::NotImplemented {
                feature: "CORRELATE WITH".into(),
                message: "CORRELATE WITH requires redesign. \
                           Use a manual JOIN between the two data sources with a time-window condition."
                    .into(),
            }),
            CompoundClause::ShowRecording => Err(ROSQLError::NotImplemented {
                feature: "SHOW RECORDING".into(),
                message: "SHOW RECORDING is deprecated. Use `FROM recordings` or \
                           `FROM recordings WHERE topic = '/camera/image_raw'` for topic-filtered \
                           recording queries."
                    .into(),
            }),
            CompoundClause::ShowJoints => self.compile_show_joints(cq),
            CompoundClause::During {
                inner_source,
                inner_conditions,
                inner_time_range,
            } => self.compile_during(inner_source, inner_conditions, inner_time_range, cq),
            CompoundClause::ShowTopics => self.compile_show_topics(cq),
            CompoundClause::ShowNodes => self.compile_show_nodes(cq),
            CompoundClause::ShowNodeGraph => self.compile_show_node_graph(cq),
        }
    }

    /// Compile scope filters to a list of WHERE predicates.
    fn compile_scope_filters(&self, scope: &QueryScope) -> Vec<String> {
        let mut parts = Vec::new();
        let res = "resource_attributes";
        if let Some(ref robot) = scope.robot {
            match robot {
                RobotScope::Single(id) => {
                    parts.push(format!(
                        "{} = '{id}'",
                        self.dialect.json_access(res, "robot.id")
                    ));
                }
                RobotScope::Fleet => {} // no filter — all robots
            }
        }
        if let Some(ref ver) = scope.version {
            parts.push(format!(
                "{} = '{ver}'",
                self.dialect.json_access(res, "service.version")
            ));
        }
        if let Some(ref env) = scope.environment {
            parts.push(format!(
                "{} = '{env}'",
                self.dialect.json_access(res, "deployment.environment")
            ));
        }
        if let Some(ref sess) = scope.session {
            parts.push(format!(
                "{} = '{sess}'",
                self.dialect.json_access(res, "ros.session.id")
            ));
        }
        parts
    }

    /// `TRACE 'trace_id'` — recursive CTE span tree walk.
    fn compile_trace(&self, trace_id: &str, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let tid = self.col("trace_id");
        let psid = self.col("parent_span_id");
        let sid = self.col("span_id");

        // Seed: root spans for this trace (parent_span_id is empty string or NULL)
        let mut seed_where = format!("{tid} = '{trace_id}' AND ({psid} = '' OR {psid} IS NULL)");
        if let Some(ref scope) = cq.scope {
            for f in self.compile_scope_filters(scope) {
                seed_where.push_str(&format!(" AND {f}"));
            }
        }

        let sql = format!(
            "WITH RECURSIVE trace_tree AS (\
             SELECT * FROM {tbl} WHERE {seed_where} \
             UNION ALL \
             SELECT t.* FROM {tbl} t \
             JOIN trace_tree r ON t.{psid} = r.{sid}\
             ) SELECT * FROM trace_tree ORDER BY {ts}",
            ts = self.col("timestamp")
        );
        Ok(sql)
    }

    /// `MESSAGE FLOW FROM TOPIC '...' [TO NODE '...' | TO TOPIC '...'] [SHOW ...]`
    fn compile_message_flow(
        &self,
        from_topic: &str,
        to_target: &Option<FlowTarget>,
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

        // Seed filter: spans on the source topic
        let mut seed_where = format!("{topic_attr} = '{from_topic}'");
        if let Some(ref scope) = cq.scope {
            for f in self.compile_scope_filters(scope) {
                seed_where.push_str(&format!(" AND {f}"));
            }
        }
        if let Some(ref tr) = cq.time_range {
            seed_where.push_str(&format!(" AND {}", self.compile_time_range(tr, "")?));
        }

        // Terminal filter for TO NODE / TO TOPIC
        let terminal_filter = match to_target {
            None => String::new(),
            Some(FlowTarget::Node(node)) => {
                let node_attr = self.dialect.json_access(span_attrs_col, "ros.node");
                format!(" WHERE {node_attr} = '{node}'")
            }
            Some(FlowTarget::Topic(topic)) => {
                format!(" WHERE {topic_attr} = '{topic}'")
            }
        };

        let mut sql = format!(
            "WITH RECURSIVE msg_flow AS (\
             SELECT * FROM {tbl} WHERE {seed_where} \
             UNION ALL \
             SELECT t.* FROM {tbl} t \
             JOIN msg_flow f ON t.{psid} = f.{sid}\
             ) SELECT * FROM msg_flow{terminal_filter}"
        );
        sql.push_str(&self.compile_compound_suffix(cq)?);
        Ok(sql)
    }

    /// `SHOW DEPLOYMENTS` — distinct version/environment deployment history.
    fn compile_show_deployments(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let res = "resource_attributes";
        let ver = self.dialect.json_access(res, "service.version");
        let env = self.dialect.json_access(res, "deployment.environment");
        let ts = self.col("timestamp");

        let mut where_parts = Vec::new();
        if let Some(ref scope) = cq.scope {
            where_parts.extend(self.compile_scope_filters(scope));
        }
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "")?);
        }
        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };

        Ok(format!(
            "SELECT {ver} AS version, {env} AS environment, \
             MIN({ts}) AS first_seen, MAX({ts}) AS last_seen \
             FROM {tbl}{where_clause} \
             GROUP BY {ver}, {env} \
             ORDER BY last_seen DESC"
        ))
    }

    /// `SHOW SPAN SUMMARY` — aggregate span stats by name.
    fn compile_show_span_summary(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let sn = self.col("span_name");
        let dur = self.col("duration");
        let _ = self.col("timestamp"); // satisfy field_registry usage; not used in output

        let mut where_parts = Vec::new();
        if let Some(ref scope) = cq.scope {
            where_parts.extend(self.compile_scope_filters(scope));
        }
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "")?);
        }
        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };

        Ok(format!(
            "SELECT {sn} AS span_name, COUNT(*) AS count, \
             AVG({dur}) AS avg_duration, MAX({dur}) AS max_duration \
             FROM {tbl}{where_clause} \
             GROUP BY {sn} \
             ORDER BY avg_duration DESC"
        ))
    }

    /// `SHOW PLANS [FOR TRACE 'id']` — plan-related spans, optionally filtered by trace.
    fn compile_show_plans(
        &self,
        trace_id: &Option<String>,
        cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let span_attrs_col = self
            .registry
            .resolve("topic")
            .map(|f| f.column.as_str())
            .unwrap_or("span_attributes");
        let plan_attr = self.dialect.json_access(span_attrs_col, "ros.plan.id");
        let tid = self.col("trace_id");
        let ts = self.col("timestamp");

        let mut where_parts = vec![format!("{plan_attr} IS NOT NULL")];
        if let Some(ref id) = trace_id {
            where_parts.push(format!("{tid} = '{id}'"));
        }
        if let Some(ref scope) = cq.scope {
            where_parts.extend(self.compile_scope_filters(scope));
        }
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "")?);
        }

        Ok(format!(
            "SELECT * FROM {tbl} WHERE {} ORDER BY {ts}",
            where_parts.join(" AND ")
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

    /// `SHOW TOPICS` — topic activity summary from span attributes.
    fn compile_show_topics(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let ts = self.col("timestamp");
        let span_attrs = self
            .registry
            .resolve("topic")
            .map(|f| f.column.as_str())
            .unwrap_or("span_attributes");
        // Use json_access for WHERE/GROUP BY and json_access_text for SELECT aliases
        // to avoid DuckDB type cast errors when ordering/comparing extracted values.
        let topic_col = self.dialect.json_access(span_attrs, "ros.topic");
        let topic_col_text = self.dialect.json_access_text(span_attrs, "ros.topic");
        let msg_type_col = self
            .dialect
            .json_access_text(span_attrs, "ros.message_type");
        let pub_node_col = self
            .dialect
            .json_access_text(span_attrs, "ros.publisher_node");
        let sub_node_col = self
            .dialect
            .json_access_text(span_attrs, "ros.subscriber_node");

        // Dialect-aware timestamp difference for avg_rate_hz calculation
        let diff_secs = self
            .dialect
            .timestamp_diff_seconds(&format!("MAX({ts})"), &format!("MIN({ts})"));
        let age_ms = self
            .dialect
            .timestamp_diff_seconds(self.dialect.now_expr(), &format!("MAX({ts})"));

        let mut where_parts = vec![format!("{topic_col_text} IS NOT NULL")];
        if let Some(ref scope) = cq.scope {
            where_parts.extend(self.compile_scope_filters(scope));
        }
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "")?);
        }
        let where_clause = format!(" WHERE {}", where_parts.join(" AND "));

        Ok(format!(
            "SELECT {topic_col_text} AS topic_name, \
             MAX({msg_type_col}) AS message_type, \
             COUNT(*) * 1.0 / NULLIF({diff_secs}, 0) AS avg_rate_hz, \
             COUNT(DISTINCT {pub_node_col}) AS publishers, \
             COUNT(DISTINCT {sub_node_col}) AS subscribers, \
             ({age_ms}) * 1000 AS last_message_age_ms \
             FROM {tbl}{where_clause} \
             GROUP BY {topic_col} \
             ORDER BY topic_name"
        ))
    }

    /// `SHOW NODES` — node activity summary from span attributes.
    fn compile_show_nodes(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let ts = self.col("timestamp");
        let span_attrs = self
            .registry
            .resolve("topic")
            .map(|f| f.column.as_str())
            .unwrap_or("span_attributes");
        let node_col = self.dialect.json_access(span_attrs, "ros.node");
        let node_col_text = self.dialect.json_access_text(span_attrs, "ros.node");
        let topic_col_text = self.dialect.json_access_text(span_attrs, "ros.topic");
        let pub_node_col_text = self
            .dialect
            .json_access_text(span_attrs, "ros.publisher_node");
        let sub_node_col_text = self
            .dialect
            .json_access_text(span_attrs, "ros.subscriber_node");
        let status_col = self.col("status_code");

        let mut where_parts = vec![format!("{node_col_text} IS NOT NULL")];
        if let Some(ref scope) = cq.scope {
            where_parts.extend(self.compile_scope_filters(scope));
        }
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "")?);
        }
        let where_clause = format!(" WHERE {}", where_parts.join(" AND "));

        Ok(format!(
            "SELECT {node_col_text} AS node_name, \
             COUNT(DISTINCT CASE WHEN {pub_node_col_text} = {node_col_text} THEN {topic_col_text} END) AS topics_published, \
             COUNT(DISTINCT CASE WHEN {sub_node_col_text} = {node_col_text} THEN {topic_col_text} END) AS topics_subscribed, \
             COUNT(CASE WHEN {status_col} = 'ERROR' THEN 1 END) AS error_count, \
             MAX({ts}) AS last_seen \
             FROM {tbl}{where_clause} \
             GROUP BY {node_col} \
             ORDER BY last_seen DESC"
        ))
    }

    /// `SHOW NODE GRAPH` — topic/node edges for graph visualisation.
    fn compile_show_node_graph(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let tbl = self.qtable("otel_traces");
        let span_attrs = self
            .registry
            .resolve("topic")
            .map(|f| f.column.as_str())
            .unwrap_or("span_attributes");
        let pub_node_text = self
            .dialect
            .json_access_text(span_attrs, "ros.publisher_node");
        let sub_node_text = self
            .dialect
            .json_access_text(span_attrs, "ros.subscriber_node");
        let topic_text = self.dialect.json_access_text(span_attrs, "ros.topic");

        let mut where_parts = vec![
            format!("{pub_node_text} IS NOT NULL"),
            format!("{sub_node_text} IS NOT NULL"),
            format!("{topic_text} IS NOT NULL"),
        ];
        if let Some(ref scope) = cq.scope {
            where_parts.extend(self.compile_scope_filters(scope));
        }
        if let Some(ref tr) = cq.time_range {
            where_parts.push(self.compile_time_range(tr, "")?);
        }
        let where_clause = format!(" WHERE {}", where_parts.join(" AND "));

        Ok(format!(
            "SELECT DISTINCT {pub_node_text} AS source_node, \
             {sub_node_text} AS target_node, \
             {topic_text} AS topic \
             FROM {tbl}{where_clause} \
             ORDER BY source_node, topic"
        ))
    }

    /// `SHOW JOINTS [FOR ROBOT ...]` — joint map query from `robot_joint_map`.
    fn compile_show_joints(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let tbl = self.qtable("robot_joint_map");

        let mut where_parts = Vec::new();
        if let Some(ref scope) = cq.scope {
            if let Some(ref robot) = scope.robot {
                if let RobotScope::Single(id) = robot {
                    where_parts.push(format!("robot_ids @> ARRAY['{id}']"));
                }
            }
            if let Some(ref ver) = scope.version {
                where_parts.push(format!("version = '{ver}'"));
            }
        }
        // Validity window: if a time range is given, filter on valid_from/valid_to
        if let Some(ref tr) = cq.time_range {
            let ts = self.compile_time_range_timestamp(tr)?;
            where_parts.push(format!(
                "valid_from <= {ts} AND (valid_to IS NULL OR valid_to > {ts})"
            ));
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };

        Ok(format!(
            "SELECT robot_model, version, robot_ids, \
             jsonb_object_keys(joint_map) AS joint_name, \
             (joint_map->jsonb_object_keys(joint_map)->>'index')::INT AS index, \
             joint_map->jsonb_object_keys(joint_map)->>'type' AS joint_type, \
             (joint_map->jsonb_object_keys(joint_map)->'limits'->>'lower')::FLOAT AS lower_limit, \
             (joint_map->jsonb_object_keys(joint_map)->'limits'->>'upper')::FLOAT AS upper_limit \
             FROM {tbl}{where_clause} \
             ORDER BY robot_model, index"
        ))
    }

    /// `PATH DEVIATION [PLAN N] FOR TRACE|ROBOT ...` — compare planned vs actual path.
    fn compile_path_deviation(
        &self,
        target: &DeviationTarget,
        plan_index: Option<i64>,
        cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let tbl = self.qtable("topic_messages");
        let plan_offset = match plan_index.unwrap_or(-1) {
            -1 => 0i64,       // latest plan = most recent = OFFSET 0 after DESC sort
            n if n >= 0 => n, // first plan = OFFSET 0, second = OFFSET 1, etc.
            n => -n - 1,      // negative indexing from end
        };

        let mut target_filters = Vec::new();
        match target {
            DeviationTarget::Trace(id) => {
                target_filters.push(format!("trace_id = '{id}'"));
            }
            DeviationTarget::Robot(id) => {
                target_filters.push(format!("robot_id = '{id}'"));
            }
        }
        if let Some(ref tr) = cq.time_range {
            target_filters.push(self.compile_time_range(tr, "")?);
        }
        let target_where = if target_filters.is_empty() {
            String::new()
        } else {
            format!(" AND {}", target_filters.join(" AND "))
        };

        // CTE 1: planned path (one nav plan, latest by default)
        // CTE 2: actual poses from /odom
        // CTE 3: per-pose lateral deviation (distance to nearest planned waypoint)
        let fields = self.col("fields");
        Ok(format!(
            "WITH planned_path AS (\
             SELECT timestamp, \
             {fields}->>'pose.pose.position.x' AS x, \
             {fields}->>'pose.pose.position.y' AS y \
             FROM {tbl} \
             WHERE topic_name = '/plan'{target_where} \
             ORDER BY timestamp DESC LIMIT 1 OFFSET {plan_offset}\
             ), actual_poses AS (\
             SELECT timestamp, \
             {fields}->>'pose.pose.position.x' AS actual_x, \
             {fields}->>'pose.pose.position.y' AS actual_y \
             FROM {tbl} \
             WHERE topic_name = '/odom'{target_where} \
             ORDER BY timestamp\
             ), deviations AS (\
             SELECT a.timestamp, \
             a.actual_x::FLOAT AS actual_x, a.actual_y::FLOAT AS actual_y, \
             p.x::FLOAT AS planned_x, p.y::FLOAT AS planned_y, \
             SQRT(POWER(a.actual_x::FLOAT - p.x::FLOAT, 2) + \
                  POWER(a.actual_y::FLOAT - p.y::FLOAT, 2)) AS lateral_deviation_m \
             FROM actual_poses a \
             CROSS JOIN LATERAL (\
               SELECT x, y FROM planned_path ORDER BY ABS(EXTRACT(EPOCH FROM \
               (planned_path.timestamp - a.timestamp))) LIMIT 1\
             ) p\
             ) SELECT \
             MAX(lateral_deviation_m) AS max_lateral_deviation_m, \
             AVG(lateral_deviation_m) AS mean_lateral_deviation_m, \
             COUNT(*) AS actual_pose_count, \
             (SELECT COUNT(*) FROM planned_path) AS planned_waypoint_count, \
             MIN(timestamp) AS start_time, MAX(timestamp) AS end_time \
             FROM deviations"
        ))
    }

    /// `JOINT DEVIATION FOR TRACE|ROBOT ...` — compare planned vs actual joint positions.
    fn compile_joint_deviation(
        &self,
        target: &DeviationTarget,
        cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let tbl = self.qtable("topic_messages");
        let fields = self.col("fields");

        let mut target_filters = Vec::new();
        match target {
            DeviationTarget::Trace(id) => {
                target_filters.push(format!("trace_id = '{id}'"));
            }
            DeviationTarget::Robot(id) => {
                target_filters.push(format!("robot_id = '{id}'"));
            }
        }
        if let Some(ref tr) = cq.time_range {
            target_filters.push(self.compile_time_range(tr, "")?);
        }
        let target_where = if target_filters.is_empty() {
            String::new()
        } else {
            format!(" AND {}", target_filters.join(" AND "))
        };

        Ok(format!(
            "WITH planned_joints AS (\
             SELECT timestamp, \
             {fields}->>'joint_names' AS joint_names, \
             {fields}->>'positions' AS planned_positions \
             FROM {tbl} \
             WHERE topic_name = '/joint_trajectory'{target_where} \
             ORDER BY timestamp\
             ), actual_joints AS (\
             SELECT timestamp, \
             {fields}->>'name' AS joint_names, \
             {fields}->>'position' AS actual_positions \
             FROM {tbl} \
             WHERE topic_name = '/joint_states'{target_where} \
             ORDER BY timestamp\
             ) SELECT \
             a.timestamp, \
             a.joint_names, \
             a.actual_positions, \
             p.planned_positions, \
             MAX(ABS(\
               (a.actual_positions::JSONB->>0)::FLOAT - (p.planned_positions::JSONB->>0)::FLOAT\
             )) AS max_joint_error_rad, \
             AVG(ABS(\
               (a.actual_positions::JSONB->>0)::FLOAT - (p.planned_positions::JSONB->>0)::FLOAT\
             )) AS mean_joint_error_rad \
             FROM actual_joints a \
             CROSS JOIN LATERAL (\
               SELECT planned_positions FROM planned_joints p2 \
               ORDER BY ABS(EXTRACT(EPOCH FROM (p2.timestamp - a.timestamp))) LIMIT 1\
             ) p \
             GROUP BY a.timestamp, a.joint_names, a.actual_positions, p.planned_positions \
             ORDER BY a.timestamp"
        ))
    }

    /// `ANOMALY(field) COMPARED TO <baseline>` — two-phase CTE statistical anomaly detection.
    fn compile_anomaly(
        &self,
        field: &str,
        compared_to: &Baseline,
        data_source: Option<&DataSource>,
        cq: &CompoundQuery,
    ) -> Result<String, ROSQLError> {
        let table = match data_source {
            Some(ds) => self.resolve_table(ds)?,
            None => self.qtable("otel_traces"),
        };
        let resolved_field = self.resolve_column(field, &table)?;

        // Build the facet dimension (GROUP BY column)
        let facet_col = match &cq.facet {
            Some(f) => self.resolve_column(&f.dimension, &table)?,
            None => "1".into(), // no facet — aggregate everything together
        };

        // Current time filter
        let current_filter = if let Some(ref tr) = cq.time_range {
            self.compile_time_range(tr, "")?
        } else {
            "1=1".into()
        };

        // Baseline time filter
        let baseline_filter = self.compile_baseline_time_filter(compared_to)?;

        // Scope filters
        let mut scope_parts = Vec::new();
        if let Some(ref scope) = cq.scope {
            scope_parts.extend(self.compile_scope_filters(scope));
        }
        let scope_clause = if scope_parts.is_empty() {
            String::new()
        } else {
            format!(" AND {}", scope_parts.join(" AND "))
        };

        // p95 expression
        let p95 = self.dialect.percentile_cont(0.95, &resolved_field);

        Ok(format!(
            "WITH current_stats AS (\
             SELECT {facet_col} AS facet_dim, \
             AVG({resolved_field}) AS current_avg, \
             {p95} AS current_p95, \
             STDDEV({resolved_field}) AS current_stddev, \
             COUNT(*) AS current_count \
             FROM {table} \
             WHERE ({current_filter}){scope_clause} \
             GROUP BY {facet_col}\
             ), baseline_stats AS (\
             SELECT {facet_col} AS facet_dim, \
             AVG({resolved_field}) AS baseline_avg, \
             {p95} AS baseline_p95, \
             STDDEV({resolved_field}) AS baseline_stddev, \
             COUNT(*) AS baseline_count \
             FROM {table} \
             WHERE ({baseline_filter}){scope_clause} \
             GROUP BY {facet_col}\
             ) SELECT \
             c.facet_dim, \
             c.current_avg, c.current_p95, c.current_count, \
             b.baseline_avg, b.baseline_p95, b.baseline_count, \
             (c.current_avg - b.baseline_avg) / NULLIF(b.baseline_stddev, 0) AS z_score, \
             ABS((c.current_avg - b.baseline_avg) / NULLIF(b.baseline_stddev, 0)) > 2 \
               AS is_anomalous, \
             CASE \
               WHEN c.current_avg > b.baseline_avg THEN 'higher' \
               WHEN c.current_avg < b.baseline_avg THEN 'lower' \
               ELSE 'normal' \
             END AS direction \
             FROM current_stats c \
             LEFT JOIN baseline_stats b ON c.facet_dim = b.facet_dim \
             ORDER BY ABS((c.current_avg - b.baseline_avg) / \
               NULLIF(b.baseline_stddev, 0)) DESC NULLS LAST"
        ))
    }

    /// Convert a `Baseline` to a SQL WHERE time predicate for the baseline window.
    fn compile_baseline_time_filter(&self, baseline: &Baseline) -> Result<String, ROSQLError> {
        let ts = self.col("timestamp");
        match baseline {
            Baseline::LastWeek => Ok(format!(
                "{ts} >= {} AND {ts} < {}",
                self.dialect.interval_ago(14.0, "days"),
                self.dialect.interval_ago(7.0, "days"),
            )),
            Baseline::Last24Hours => Ok(format!(
                "{ts} >= {} AND {ts} < {}",
                self.dialect.interval_ago(48.0, "hours"),
                self.dialect.interval_ago(24.0, "hours"),
            )),
            Baseline::Fleet => Ok("1=1".into()), // no time restriction — fleet-wide baseline
            other => Err(ROSQLError::CompilationError {
                message: format!(
                    "ANOMALY COMPARED TO {:?} is not supported. \
                     Use COMPARED TO last week, COMPARED TO last 24 hours, or COMPARED TO fleet.",
                    other
                ),
            }),
        }
    }

    /// Extract the timestamp expression from a TimeRange for point-in-time comparisons.
    fn compile_time_range_timestamp(&self, tr: &TimeRange) -> Result<String, ROSQLError> {
        match tr {
            TimeRange::Since(te) => self.compile_time_expr(te),
            TimeRange::Between { start, .. } => self.compile_time_expr(start),
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────

    /// Appends ORDER BY / LIMIT / OFFSET to a compound query.
    /// Time range and conditions are handled inline by the specific compile_* functions.
    fn compile_compound_suffix(&self, cq: &CompoundQuery) -> Result<String, ROSQLError> {
        let mut parts = Vec::new();

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
            Condition::Within {
                field: _,
                radius,
                center,
            } => {
                let r = radius.si_value;
                match center {
                    GeospatialCenter::Gps(lat, lon) => {
                        // Inline Haversine SQL: sqrt((2 * R * asin(sqrt(haversin(dlat) + cos(lat1)*cos(lat2)*haversin(dlon)))))
                        // Field paths: fields->'pose'->'pose'->'position'->'x' = longitude,
                        //              fields->'pose'->'pose'->'position'->'y' = latitude
                        // For nav_msgs/Odometry stored in topic_messages.fields JSONB.
                        let fields = self.col("fields");
                        let lat_col = format!("({fields}->>'latitude')::FLOAT");
                        let lon_col = format!("({fields}->>'longitude')::FLOAT");
                        Ok(format!(
                            "(6371000.0 * 2 * ASIN(SQRT(\
                             POWER(SIN(RADIANS(({lat_col} - {lat}) / 2.0)), 2) + \
                             COS(RADIANS({lat})) * COS(RADIANS({lat_col})) * \
                             POWER(SIN(RADIANS(({lon_col} - {lon}) / 2.0)), 2)\
                             ))) <= {r}"
                        ))
                    }
                    GeospatialCenter::Local(cx, cy) => {
                        // Euclidean distance in local frame using pose.pose.position.x / .y
                        let fields = self.col("fields");
                        let x_col = format!("({fields}->>'pose.pose.position.x')::FLOAT");
                        let y_col = format!("({fields}->>'pose.pose.position.y')::FLOAT");
                        Ok(format!(
                            "SQRT(POWER({x_col} - {cx}, 2) + POWER({y_col} - {cy}, 2)) <= {r}"
                        ))
                    }
                }
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
            Expr::FieldAccess { base, key } => {
                // Detect array index notation: "name[N]" → json_array_access
                if let Some(bracket) = key.find('[') {
                    if let Some(close) = key.rfind(']') {
                        if close > bracket {
                            let field_name = &key[..bracket];
                            let index_str = &key[bracket + 1..close];
                            if let Ok(index) = index_str.parse::<usize>() {
                                return Ok(self.dialect.json_array_access(base, field_name, index));
                            }
                        }
                    }
                }
                Ok(self.dialect.json_access(base, key))
            }
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
    fn trace_recursive_cte() {
        let sql = compile_pg("TRACE 'abc123'");
        assert!(sql.contains("WITH RECURSIVE trace_tree"), "got: {sql}");
        assert!(sql.contains("abc123"), "got: {sql}");
        assert!(sql.contains("parent_span_id"), "got: {sql}");
        assert!(sql.contains("otel_traces"), "got: {sql}");
    }

    #[test]
    fn trace_with_robot_scope() {
        let sql = compile_pg("TRACE 'abc123' FOR ROBOT 'r1'");
        assert!(sql.contains("WITH RECURSIVE trace_tree"), "got: {sql}");
        assert!(sql.contains("robot.id"), "got: {sql}");
        assert!(sql.contains("r1"), "got: {sql}");
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
    fn anomaly_compiles_two_cte() {
        // ANOMALY now compiles to a two-phase CTE (current_stats + baseline_stats).
        let sql =
            compile_pg("ANOMALY(duration) COMPARED TO last week FACET robot_id SINCE 7 days ago");
        assert!(sql.contains("current_stats"), "got: {sql}");
        assert!(sql.contains("baseline_stats"), "got: {sql}");
        assert!(sql.contains("z_score"), "got: {sql}");
        assert!(sql.contains("is_anomalous"), "got: {sql}");
        assert!(sql.contains("direction"), "got: {sql}");
    }

    #[test]
    fn anomaly_missing_compared_to_is_parse_error() {
        // ANOMALY without COMPARED TO is now a parse error, not a NotImplemented.
        let result = crate::parse("ANOMALY(duration) SINCE 1 hour ago");
        assert!(result.is_err(), "expected parse error");
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(|e| matches!(e, ROSQLError::ParseError { message, .. } if message.contains("COMPARED TO"))),
            "got: {errs:?}"
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
    fn path_deviation_compiles() {
        // PATH DEVIATION now compiles to a multi-CTE SQL query.
        let sql = compile_pg("PATH DEVIATION FOR ROBOT 'r1' SINCE yesterday");
        assert!(sql.contains("planned_path"), "got: {sql}");
        assert!(sql.contains("actual_poses"), "got: {sql}");
        assert!(sql.contains("lateral_deviation_m"), "got: {sql}");
        assert!(sql.contains("/plan"), "got: {sql}");
        assert!(sql.contains("/odom"), "got: {sql}");
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
