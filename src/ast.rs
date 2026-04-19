//! Typed AST for ROSQL queries.
//!
//! All types are native Rust — no proto dependency in the core lib.
//! Every type derives `Debug, Clone, PartialEq, Serialize, Deserialize`.

use serde::{Deserialize, Serialize};

// ===========================================================================
// Top-level query
// ===========================================================================

/// A parsed ROSQL query — either standard SQL-like form or pipeline form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Query {
    Standard(ROSQLQuery),
    Pipeline(PipelineQuery),
    Compound(CompoundQuery),
}

/// Standard SQL-shaped query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ROSQLQuery {
    pub selections: Vec<Selection>,
    pub data_source: DataSource,
    pub scope: Option<QueryScope>,
    pub conditions: Option<Condition>,
    pub facet: Option<FacetClause>,
    pub time_range: Option<TimeRange>,
    pub time_basis: Option<TimeBasis>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub output_format: Option<OutputFormat>,
    pub baseline: Option<Baseline>,
    pub timeseries: Option<TimeseriesClause>,
    pub enrichments: Vec<EnrichmentClause>,
    pub during: Option<CompoundClause>,
}

/// Pipeline query: `FROM source | WHERE ... | FACET ...`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineQuery {
    pub stages: Vec<PipelineStage>,
}

/// A single stage in a pipeline query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PipelineStage {
    From(DataSource),
    Select(Vec<Selection>),
    Where(Condition),
    Facet(FacetClause),
    Since(TimeRange),
    Using(TimeBasis),
    OrderBy(OrderBy),
    Limit(u64),
    Offset(u64),
    Format(OutputFormat),
    CompareTo(Baseline),
    ForScope(QueryScope),
    CompoundClause(CompoundClause),
    Timeseries(TimeseriesClause),
    EnrichWith(EnrichmentClause),
}

/// A top-level compound query (MESSAGE FLOW, HEALTH(), TRACE, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompoundQuery {
    pub clause: CompoundClause,
    pub scope: Option<QueryScope>,
    pub time_range: Option<TimeRange>,
    pub time_basis: Option<TimeBasis>,
    pub conditions: Option<Condition>,
    pub facet: Option<FacetClause>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub output_format: Option<OutputFormat>,
    pub baseline: Option<Baseline>,
}

// ===========================================================================
// Data sources
// ===========================================================================

/// The FROM target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataSource {
    Logs,
    SystemLogs,
    Traces,
    Metrics,
    Diagnostics,
    Topics,
    Tf,
    Heartbeats,
    Recordings,
    Events,
    /// Well-known topic alias (e.g. `FROM odom`).
    TopicAlias(TopicAlias),
}

/// Well-known ROS2 topic aliases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TopicAlias {
    /// `/odom`
    Odom,
    /// `/joint_states`
    JointStates,
    /// `/battery_state`
    Battery,
    /// `/cmd_vel`
    CmdVel,
    /// `/imu/data`
    Imu,
}

impl TopicAlias {
    /// The actual ROS2 topic name this alias maps to.
    pub fn topic_name(&self) -> &'static str {
        match self {
            TopicAlias::Odom => "/odom",
            TopicAlias::JointStates => "/joint_states",
            TopicAlias::Battery => "/battery_state",
            TopicAlias::CmdVel => "/cmd_vel",
            TopicAlias::Imu => "/imu/data",
        }
    }
}

// ===========================================================================
// Selections
// ===========================================================================

/// A single item in the SELECT list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Selection {
    /// `*`
    Star,
    /// A field reference (possibly dotted, e.g. `ros.node`).
    Field(String),
    /// An aggregation call (e.g. `AVG(duration)`).
    Aggregation(AggregationCall),
    /// An aliased expression (e.g. `AVG(duration) AS avg_dur`).
    Aliased { expr: Box<Selection>, alias: String },
}

/// An aggregation function call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregationCall {
    pub function: AggregationFn,
    pub args: Vec<Expr>,
}

/// All supported aggregation/function names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationFn {
    // Standard
    Count,
    Sum,
    Avg,
    Min,
    Max,
    Percentile,
    Stddev,
    // Time-series
    Rate,
    Delta,
    Derivative,
    MovingAvg,
    // Robotics-specific
    TopicRate,
    NodeStatus,
    Expected,
    ActionSuccessRate,
    Uptime,
    // ClickHouse approximate (platform)
    ApproxCountDistinct,
    ApproxPercentile,
}

// ===========================================================================
// Expressions
// ===========================================================================

/// A value expression used in conditions, selections, and function arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// A field reference (e.g. `duration`, `ros.node`).
    Field(String),
    /// A literal value.
    Literal(Literal),
    /// A numeric value with a unit (e.g. `500 ms`).
    UnitValue(UnitValue),
    /// An aggregation call.
    Aggregation(AggregationCall),
    /// Map/bracket field access (e.g. `fields['my_value']`).
    FieldAccess { base: String, key: String },
    /// Arithmetic: `expr op expr`
    BinaryOp {
        left: Box<Expr>,
        op: ArithmeticOp,
        right: Box<Expr>,
    },
}

/// Arithmetic operators in expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// A literal value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

/// A numeric value with a physical unit, including SI normalisation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitValue {
    /// The value as written by the user.
    pub raw_value: f64,
    /// The unit symbol as written (e.g. "ms").
    pub unit: String,
    /// The SI-normalised value (e.g. 0.5 for 500 ms).
    pub si_value: f64,
    /// The SI base unit symbol (e.g. "s").
    pub si_unit: String,
}

// ===========================================================================
// Conditions
// ===========================================================================

/// A boolean condition (WHERE clause).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    Comparison {
        left: Expr,
        op: ComparisonOp,
        right: Expr,
    },
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    IsNull(Expr),
    IsNotNull(Expr),
    In {
        expr: Expr,
        values: Vec<Expr>,
    },
    Like {
        expr: Expr,
        pattern: String,
    },
    Between {
        expr: Expr,
        low: Box<Expr>,
        high: Box<Expr>,
    },
    /// `<expr> WITHIN <radius> OF (<lat>, <lon>)` or `OF POSITION (<x>, <y>)`
    Within {
        field: Expr,
        radius: UnitValue,
        center: GeospatialCenter,
    },
}

/// Center point for a WITHIN geospatial condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeospatialCenter {
    /// GPS coordinates (lat, lon) — Haversine distance.
    Gps(f64, f64),
    /// Local frame position (x, y) — Euclidean distance.
    Local(f64, f64),
}

/// Comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
}

// ===========================================================================
// Time
// ===========================================================================

/// A time range constraint (SINCE or BETWEEN).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeRange {
    Since(TimeExpr),
    Between { start: TimeExpr, end: TimeExpr },
}

/// A single time expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeExpr {
    /// Relative time (e.g. `30 minutes ago`, `yesterday`).
    Relative(RelativeTime),
    /// Absolute RFC 3339 timestamp (e.g. `'2026-03-18T14:00:00Z'`).
    Absolute(String),
    /// Unix epoch timestamp.
    Epoch(UnixEpoch),
    /// Lifecycle anchor (e.g. `last deployment`).
    Anchor(LifecycleAnchor),
}

/// A relative time expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelativeTime {
    pub amount: f64,
    pub unit: String, // "minutes", "hours", "days", etc.
}

/// A unix epoch timestamp, disambiguated by digit count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnixEpoch {
    /// 10-digit: seconds since epoch.
    Seconds(u64),
    /// 13-digit: milliseconds since epoch.
    Milliseconds(u64),
    /// 19-digit: nanoseconds since epoch.
    Nanoseconds(u64),
}

/// Lifecycle anchors for time expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LifecycleAnchor {
    LastDeployment,
    LastRobotRestart,
    LastActionFailure,
    LastTopicDrop,
    LastDiagnosticWarning,
}

/// Time basis selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeBasis {
    RosTime,
    WallTime,
}

// ===========================================================================
// Robot / query scoping
// ===========================================================================

/// Robot scope (FOR ROBOT / FOR FLEET).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RobotScope {
    Single(String),
    Fleet,
}

/// Composable query scope — all dimensions are optional and can be combined.
///
/// ```sql
/// FOR ROBOT 'robot_42' FOR VERSION '2.3.1' FOR ENVIRONMENT 'production'
/// ```
///
/// Compile targets:
/// - `FOR ROBOT`       → `resource_attributes->>'robot.id'`
/// - `FOR VERSION`     → `resource_attributes->>'service.version'`
/// - `FOR ENVIRONMENT` → `resource_attributes->>'deployment.environment'`
/// - `FOR SESSION`     → `resource_attributes->>'ros.session.id'`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryScope {
    pub robot: Option<RobotScope>,
    pub version: Option<String>,
    pub environment: Option<String>,
    pub session: Option<String>,
}

impl QueryScope {
    pub fn empty() -> Self {
        QueryScope {
            robot: None,
            version: None,
            environment: None,
            session: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.robot.is_none()
            && self.version.is_none()
            && self.environment.is_none()
            && self.session.is_none()
    }
}

// ===========================================================================
// Facet, ordering, output format
// ===========================================================================

/// FACET clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacetClause {
    pub dimension: String,
}

/// ORDER BY clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBy {
    pub field: String,
    pub direction: SortDirection,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Output format (FORMAT clause) — user-facing keyword after `FORMAT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Table,
    Timeseries,
    Scalar,
    TraceTree,
    Graph,
    Path,
}

/// Presentation-layer format hint — inferred from query shape for frontends.
///
/// Unlike `OutputFormat` (which reflects the user's explicit `FORMAT` clause),
/// `FormatHint` is automatically inferred from the query structure and tells
/// consumers how to best visualize the result (e.g. line chart, gantt, etc.).
/// An explicit `FORMAT` clause overrides inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatHint {
    /// Generic tabular display (default fallback).
    Table,
    /// Time-bucketed line chart (TIMESERIES without FACET).
    LineChart,
    /// Multi-series time-bucketed line chart (TIMESERIES with FACET).
    StackedLineChart,
    /// Bar chart (FACET without TIMESERIES, or aggregation by category).
    BarChart,
    /// Horizontal bar chart (SHOW SPAN SUMMARY).
    HorizontalBars,
    /// Gantt / waterfall chart (TRACE 'id').
    Gantt,
    /// Directed graph (MESSAGE FLOW).
    DirectedGraph,
    /// Undirected node graph (SHOW NODE GRAPH).
    NodeGraph,
    /// Metric cards for scalar aggregations.
    ScalarCards,
    /// Log viewer with severity coloring (FROM logs).
    LogTable,
    /// Recording / bag file list (FROM recordings).
    RecordingList,
}

// ===========================================================================
// Timeseries
// ===========================================================================

/// TIMESERIES interval — time-bucketed aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeseriesClause {
    /// The bucket width (e.g. `5 min`, `1 hour`).
    pub interval: UnitValue,
}

// ===========================================================================
// Enrichment
// ===========================================================================

/// ENRICH WITH clause — joins additional data into a primary query result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnrichmentClause {
    /// The data source to enrich from.
    pub source: DataSource,
    /// Explicit join key override (None = infer from source pair).
    pub join_key: Option<String>,
    /// Per-primary-row row limit (default 50).
    pub limit: Option<u64>,
    /// Disable auto-downsampling for high-frequency topic data.
    pub sample_full: bool,
}

// ===========================================================================
// Baselines
// ===========================================================================

/// COMPARE TO baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Baseline {
    LastWeek,
    /// `COMPARED TO last 24 hours`
    Last24Hours,
    Fleet,
    Robot(String),
    LastDeployment,
    CompareRobots,
    /// `COMPARE TO VERSION 'v1.2.3'`
    Version(String),
    /// `COMPARE VERSION 'v1.0' TO VERSION 'v2.0'`
    VersionPair(String, String),
}

/// Target selector for PATH DEVIATION and JOINT DEVIATION queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviationTarget {
    /// `FOR TRACE 'trace_id'`
    Trace(String),
    /// `FOR ROBOT 'robot_id'`
    Robot(String),
}

// ===========================================================================
// Compound clauses
// ===========================================================================

/// Target qualifier for MESSAGE FLOW.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FlowTarget {
    /// `TO NODE '/node_name'`
    Node(String),
    /// `TO TOPIC '/topic_name'`
    Topic(String),
}

/// Compound clause types — all are open source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompoundClause {
    /// `DURING(subquery)` — cross-signal temporal correlation.
    During {
        inner_source: DataSource,
        inner_conditions: Option<Condition>,
        inner_time_range: Option<TimeRange>,
    },

    /// `TRACE 'trace_id'` — recursive span tree walk.
    Trace { trace_id: String },

    /// `MESSAGE FLOW FROM TOPIC '/topic' [TO NODE '/node' | TO TOPIC '/topic'] [SHOW ...]`
    MessageFlow {
        from_topic: String,
        to_target: Option<FlowTarget>,
        show: Option<String>,
    },

    /// `SHOW TRACE_BREAKDOWN`
    ShowTraceBreakdown,

    /// `HEALTH() [FOR ROBOT ...]`
    Health,

    /// `ANOMALY(field) [FROM <source>] COMPARED TO <baseline>`
    Anomaly {
        field: String,
        /// Required in v0.5 — COMPARED TO baseline.
        compared_to: Baseline,
        /// Optional data source override (defaults to `otel_traces`).
        data_source: Option<DataSource>,
    },

    /// `PATH DEVIATION [PLAN <n>] FOR TRACE|ROBOT ...`
    PathDeviation {
        target: DeviationTarget,
        /// Plan index: 0 = first plan, -1 = latest (default).
        plan_index: Option<i64>,
    },

    /// `JOINT DEVIATION FOR TRACE|ROBOT ...`
    JointDeviation { target: DeviationTarget },

    /// `CORRELATE WITH <source>`
    Correlate { with_source: DataSource },

    /// `SHOW RECORDING`
    ShowRecording,

    /// `SHOW DEPLOYMENTS [FOR ROBOT ...] [SINCE ...]`
    ShowDeployments,

    /// `SHOW SPAN SUMMARY [FOR ROBOT ...] [SINCE ...]`
    ShowSpanSummary,

    /// `SHOW PLANS [FOR TRACE 'trace_id'] [FOR ROBOT ...] [SINCE ...]`
    ShowPlans { trace_id: Option<String> },

    /// `SHOW TOPICS [FOR ROBOT ...] [SINCE ...]`
    ShowTopics,

    /// `SHOW NODES [FOR ROBOT ...] [SINCE ...]`
    ShowNodes,

    /// `SHOW NODE GRAPH [FOR ROBOT ...] [SINCE ...]`
    ShowNodeGraph,

    /// `SHOW JOINTS [FOR ROBOT ...]`
    ShowJoints,
}
