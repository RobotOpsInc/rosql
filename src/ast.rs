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
    pub robot_scope: Option<RobotScope>,
    pub conditions: Option<Condition>,
    pub facet: Option<FacetClause>,
    pub time_range: Option<TimeRange>,
    pub time_basis: Option<TimeBasis>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u64>,
    pub output_format: Option<OutputFormat>,
    pub baseline: Option<Baseline>,
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
    Format(OutputFormat),
    CompareTo(Baseline),
    ForRobot(RobotScope),
    CompoundClause(CompoundClause),
}

/// A top-level compound query (MESSAGE JOURNEY, HEALTH(), TRACE, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompoundQuery {
    pub clause: CompoundClause,
    pub robot_scope: Option<RobotScope>,
    pub time_range: Option<TimeRange>,
    pub time_basis: Option<TimeBasis>,
    pub conditions: Option<Condition>,
    pub facet: Option<FacetClause>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u64>,
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
// Robot scoping
// ===========================================================================

/// Robot scope (FOR ROBOT / FOR FLEET).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RobotScope {
    Single(String),
    Fleet,
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

/// Output format (FORMAT clause).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Table,
    Timeseries,
    Scalar,
    TraceTree,
    Graph,
    Path,
}

// ===========================================================================
// Baselines
// ===========================================================================

/// COMPARE TO baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Baseline {
    LastWeek,
    Fleet,
    Robot(String),
    LastDeployment,
    CompareRobots,
}

// ===========================================================================
// Compound clauses
// ===========================================================================

/// Compound clause types — all are open source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompoundClause {
    /// `DURING(subquery)` — cross-signal temporal correlation.
    During {
        inner_source: DataSource,
        inner_conditions: Option<Condition>,
        inner_time_range: Option<TimeRange>,
    },

    /// `MESSAGE JOURNEY FOR TRACE 'trace_id'`
    MessageJourney { trace_id: String },

    /// `MESSAGE PATHS FOR TOPIC '/topic' SINCE ...`
    MessagePaths { topic: String },

    /// `MESSAGE PATH FROM TOPIC '/src' TO NODE '/dst' [SHOW ...]`
    MessagePath {
        from_topic: String,
        to_node: String,
        show: Option<String>,
    },

    /// `TRACE 'trace_id'`
    Trace { trace_id: String },

    /// `SHOW TRACE_BREAKDOWN`
    ShowTraceBreakdown,

    /// `HEALTH() [FOR ROBOT ...]`
    Health,

    /// `ANOMALY(field) [COMPARED TO ...]`
    Anomaly {
        field: String,
        compared_to: Option<Baseline>,
    },

    /// `PATH DEVIATION [FOR ROBOT ...] [SHOW ...]`
    PathDeviation { show: Option<Vec<String>> },

    /// `CORRELATE WITH <source>`
    Correlate { with_source: DataSource },

    /// `SHOW RECORDING`
    ShowRecording,
}
