//! Context-aware completion engine for ROSQL queries.
//!
//! Used by both WASM and gRPC to provide autocomplete suggestions
//! at a given cursor position in a ROSQL query string.

use serde::{Deserialize, Serialize};

/// A single completion suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Completion {
    /// The text to insert.
    pub label: String,
    /// Human-readable description.
    pub detail: String,
    /// The kind of completion.
    pub kind: CompletionKind,
}

/// The kind of a completion suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionKind {
    Keyword,
    DataSource,
    Field,
    Function,
    Unit,
}

/// Get context-aware completions at a cursor position in a ROSQL query.
pub fn get_completions(query: &str, cursor_pos: usize) -> Vec<Completion> {
    let prefix = &query[..cursor_pos.min(query.len())];
    let trimmed = prefix.trim();

    // Determine context from the last significant token
    let upper = trimmed.to_uppercase();

    if upper.is_empty() || is_query_start(&upper) {
        return query_start_completions();
    }

    if upper.ends_with("FROM ") || upper.ends_with("FROM") {
        return data_source_completions();
    }

    if upper.ends_with("WHERE ")
        || upper.ends_with("WHERE")
        || upper.ends_with("AND ")
        || upper.ends_with("OR ")
    {
        return field_completions();
    }

    if upper.ends_with("SINCE ") || upper.ends_with("SINCE") {
        return time_completions();
    }

    if upper.ends_with("COMPARE TO ") || upper.ends_with("COMPARE TO") {
        return baseline_completions();
    }

    if upper.ends_with("FORMAT ") || upper.ends_with("FORMAT") {
        return format_completions();
    }

    if upper.ends_with("USING ") || upper.ends_with("USING") {
        return time_basis_completions();
    }

    if upper.ends_with("LAST ") || upper.ends_with("LAST") {
        return lifecycle_anchor_completions();
    }

    // After a number, suggest unit suffixes
    if trimmed.chars().last().is_some_and(|c| c.is_ascii_digit()) {
        return unit_completions();
    }

    // Default: suggest keywords
    keyword_completions()
}

// ---------------------------------------------------------------------------
// Completion providers
// ---------------------------------------------------------------------------

fn query_start_completions() -> Vec<Completion> {
    vec![
        kw("SELECT", "Select specific fields"),
        kw("FROM", "Query a data source"),
        kw("MESSAGE JOURNEY", "Trace message causality graph"),
        kw("MESSAGE PATHS", "Find message paths for a topic"),
        kw("MESSAGE PATH", "Find path between topic and node"),
        kw("HEALTH()", "Derived robot health assessment"),
        kw("ANOMALY()", "Statistical anomaly detection"),
        kw("PATH DEVIATION", "Spatial trajectory analysis"),
        kw("TRACE", "Show spans for a trace ID"),
        kw("SHOW RECORDING", "Find MCAP recordings"),
        kw("CORRELATE", "Cross-signal correlation"),
    ]
}

fn data_source_completions() -> Vec<Completion> {
    vec![
        ds("logs", "ROS2 /rosout logs"),
        ds("traces", "Distributed traces and action spans"),
        ds("metrics", "System metrics and topic rates"),
        ds("diagnostics", "ROS2 /diagnostics_agg health"),
        ds("topics", "Deserialized topic messages"),
        ds("tf", "TF transform tree"),
        ds("heartbeats", "Robot liveness data"),
        ds("recordings", "MCAP recording index"),
        ds("events", "Discrete events"),
        ds("odom", "Alias: /odom topic"),
        ds("joint_states", "Alias: /joint_states topic"),
        ds("battery", "Alias: /battery_state topic"),
        ds("cmd_vel", "Alias: /cmd_vel topic"),
        ds("imu", "Alias: /imu/data topic"),
    ]
}

fn field_completions() -> Vec<Completion> {
    vec![
        field("duration", "Span duration"),
        field("status", "Span status (OK/ERROR)"),
        field("span_name", "Span name"),
        field("node", "ROS2 node name"),
        field("action_name", "ROS2 action name"),
        field("action_status", "Action result status"),
        field("topic", "ROS2 topic name"),
        field("service", "Service name"),
        field("trace_id", "Trace ID"),
        field("message", "Log message body"),
        field("severity", "Log severity"),
        field("publish_rate", "Topic publish rate"),
        field("cpu_usage", "CPU usage"),
        field("memory_usage", "Memory usage"),
        field("robot_id", "Robot identifier"),
    ]
}

fn time_completions() -> Vec<Completion> {
    vec![
        kw("yesterday", "Since yesterday"),
        kw("last deployment", "Since last deployment"),
        kw("last robot restart", "Since last robot restart"),
        kw("last action failure", "Since last action failure"),
        kw("last topic drop", "Since last topic drop"),
        kw("last diagnostic warning", "Since last diagnostic warning"),
    ]
}

fn baseline_completions() -> Vec<Completion> {
    vec![
        kw("last week", "Compare to previous week"),
        kw("fleet", "Compare to fleet average"),
        kw("last deployment", "Compare to before last deployment"),
    ]
}

fn format_completions() -> Vec<Completion> {
    vec![
        kw("table", "Columnar table output"),
        kw("timeseries", "Time-bucketed series"),
        kw("scalar", "Single aggregate value"),
        kw("trace_tree", "Nested span tree"),
        kw("graph", "Node/edge adjacency list"),
        kw("path", "Coordinate path data"),
    ]
}

fn time_basis_completions() -> Vec<Completion> {
    vec![
        kw("ROS_TIME", "Use ROS simulation time"),
        kw("WALL_TIME", "Use wall clock time"),
    ]
}

fn lifecycle_anchor_completions() -> Vec<Completion> {
    vec![
        kw("deployment", "Last deployment"),
        kw("robot restart", "Last robot restart"),
        kw("action failure", "Last action failure"),
        kw("topic drop", "Last topic drop"),
        kw("diagnostic warning", "Last diagnostic warning"),
        kw("week", "Last week (for COMPARE TO)"),
    ]
}

fn unit_completions() -> Vec<Completion> {
    vec![
        unit("ms", "Milliseconds"),
        unit("s", "Seconds"),
        unit("min", "Minutes"),
        unit("h", "Hours"),
        unit("m", "Meters"),
        unit("km", "Kilometers"),
        unit("Hz", "Hertz"),
        unit("deg", "Degrees"),
        unit("rad", "Radians"),
        unit("V", "Volts"),
        unit("B", "Bytes"),
        unit("MB", "Megabytes"),
        unit("Pa", "Pascals"),
    ]
}

fn keyword_completions() -> Vec<Completion> {
    vec![
        kw("WHERE", "Filter conditions"),
        kw("SINCE", "Time range start"),
        kw("BETWEEN", "Time range"),
        kw("FACET", "Group by dimension"),
        kw("ORDER BY", "Sort results"),
        kw("LIMIT", "Limit result count"),
        kw("FORMAT", "Output format"),
        kw("COMPARE TO", "Baseline comparison"),
        kw("FOR ROBOT", "Scope to a robot"),
        kw("FOR FLEET", "Scope to fleet"),
        kw("USING", "Time basis (ROS_TIME/WALL_TIME)"),
    ]
}

fn is_query_start(upper: &str) -> bool {
    // Empty or just whitespace
    upper.is_empty() || upper.chars().all(|c| c.is_whitespace())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kw(label: &str, detail: &str) -> Completion {
    Completion {
        label: label.into(),
        detail: detail.into(),
        kind: CompletionKind::Keyword,
    }
}

fn ds(label: &str, detail: &str) -> Completion {
    Completion {
        label: label.into(),
        detail: detail.into(),
        kind: CompletionKind::DataSource,
    }
}

fn field(label: &str, detail: &str) -> Completion {
    Completion {
        label: label.into(),
        detail: detail.into(),
        kind: CompletionKind::Field,
    }
}

fn unit(label: &str, detail: &str) -> Completion {
    Completion {
        label: label.into(),
        detail: detail.into(),
        kind: CompletionKind::Unit,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_at_start() {
        let completions = get_completions("", 0);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"SELECT"));
        assert!(labels.contains(&"FROM"));
        assert!(labels.contains(&"HEALTH()"));
    }

    #[test]
    fn completions_after_from() {
        let completions = get_completions("FROM ", 5);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"logs"));
        assert!(labels.contains(&"traces"));
        assert!(labels.contains(&"odom"));
    }

    #[test]
    fn completions_after_where() {
        let completions = get_completions("FROM traces WHERE ", 18);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"duration"));
        assert!(labels.contains(&"status"));
        assert!(labels.contains(&"node"));
    }

    #[test]
    fn completions_after_since() {
        let completions = get_completions("FROM logs SINCE ", 16);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"yesterday"));
        assert!(labels.contains(&"last deployment"));
    }

    #[test]
    fn completions_after_number() {
        let completions = get_completions("FROM traces WHERE duration > 500", 32);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"ms"));
        assert!(labels.contains(&"s"));
    }

    #[test]
    fn completions_after_format() {
        let completions = get_completions("FROM logs FORMAT ", 17);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"table"));
        assert!(labels.contains(&"timeseries"));
    }

    #[test]
    fn completions_after_compare_to() {
        let completions = get_completions("FROM logs COMPARE TO ", 21);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"last week"));
        assert!(labels.contains(&"fleet"));
    }

    #[test]
    fn completions_after_using() {
        let completions = get_completions("FROM logs USING ", 16);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"ROS_TIME"));
        assert!(labels.contains(&"WALL_TIME"));
    }
}
