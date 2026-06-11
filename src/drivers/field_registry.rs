//! Field registry — maps ROSQL field names to database column expressions.

use crate::ast::DataSource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A registry mapping ROSQL field names to their database column definitions.
///
/// Multiple definitions can be registered under the same field name when a
/// field has different representations across tables (e.g. `robot_id` is a
/// bare column on `topic_messages` but a JSON-extracted value on OTel tables).
/// `resolve_for_table` prefers the entry whose `source_table` matches; `resolve`
/// falls back to the first registered entry for backwards compatibility.
#[derive(Debug, Clone)]
pub struct FieldRegistry {
    fields: HashMap<String, Vec<FieldDef>>,
    /// Maps DataSource variants to their underlying table names.
    table_names: HashMap<String, String>,
}

/// Definition of a single queryable field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    /// The ROSQL field name (e.g. "duration", "node").
    pub name: String,
    /// The source table (e.g. "otel_traces", "otel_metrics").
    pub source_table: String,
    /// The actual column name in the database.
    pub column: String,
    /// Storage unit (e.g. "ns" for nanoseconds). Used for unit conversion.
    pub storage_unit: Option<String>,
    /// Whether this field requires JSONB/JSON map access.
    pub is_map_access: bool,
    /// The map column name (e.g. "SpanAttributes") when `is_map_access` is true.
    pub map_column: Option<String>,
    /// The key inside the map column (e.g. "ros.node").
    pub map_key: Option<String>,
    /// For metric fields, the MetricName filter value.
    pub metric_filter: Option<String>,
}

impl FieldRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            table_names: HashMap::new(),
        }
    }

    /// Register a field definition. Multiple definitions for the same field
    /// name are allowed (table-scoped overloads).
    pub fn register(&mut self, field: FieldDef) {
        self.fields
            .entry(field.name.clone())
            .or_default()
            .push(field);
    }

    /// Register a table name mapping for a data source key.
    pub fn register_table(&mut self, source_key: &str, table_name: &str) {
        self.table_names
            .insert(source_key.to_string(), table_name.to_string());
    }

    /// Resolve a ROSQL field name, preferring the definition whose
    /// `source_table` matches `table`. Falls back to the first registered
    /// definition if no table-specific entry exists.
    pub fn resolve_for_table<'a>(&'a self, field_name: &str, table: &str) -> Option<&'a FieldDef> {
        let defs = self.fields.get(field_name)?;
        // Prefer an exact table match.
        if let Some(def) = defs.iter().find(|d| d.source_table == table) {
            return Some(def);
        }
        // Fall back to the first (and historically only) entry.
        defs.first()
    }

    /// Resolve a ROSQL field name without table context.
    /// Returns the first registered definition for the name.
    pub fn resolve(&self, field_name: &str) -> Option<&FieldDef> {
        self.fields.get(field_name)?.first()
    }

    /// Get all fields defined for a given source table.
    pub fn fields_for_table(&self, table_name: &str) -> Vec<&FieldDef> {
        self.fields
            .values()
            .flatten()
            .filter(|f| f.source_table == table_name)
            .collect()
    }

    /// Get the underlying table name for a DataSource.
    pub fn table_name(&self, source: &DataSource) -> Option<&str> {
        let key = data_source_key(source);
        self.table_names.get(&key).map(|s| s.as_str())
    }
}

impl Default for FieldRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a DataSource to a string key for table name lookup.
pub fn data_source_key(source: &DataSource) -> String {
    match source {
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
        DataSource::NodeGraph => "node_graph".into(),
        DataSource::Joints => "joints".into(),
        DataSource::TopicAlias(_) => "topics".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_registered_field() {
        let mut reg = FieldRegistry::new();
        reg.register(FieldDef {
            name: "duration".into(),
            source_table: "otel_traces".into(),
            column: "Duration".into(),
            storage_unit: Some("ns".into()),
            is_map_access: false,
            map_column: None,
            map_key: None,
            metric_filter: None,
        });
        let field = reg.resolve("duration").unwrap();
        assert_eq!(field.column, "Duration");
        assert_eq!(field.storage_unit.as_deref(), Some("ns"));
    }

    #[test]
    fn resolve_unknown_field() {
        let reg = FieldRegistry::new();
        assert!(reg.resolve("nonexistent").is_none());
    }

    #[test]
    fn table_name_lookup() {
        let mut reg = FieldRegistry::new();
        reg.register_table("traces", "otel_traces");
        assert_eq!(reg.table_name(&DataSource::Traces), Some("otel_traces"));
    }

    #[test]
    fn topic_alias_resolves_to_topics_table() {
        let mut reg = FieldRegistry::new();
        reg.register_table("topics", "topic_messages");
        assert_eq!(
            reg.table_name(&DataSource::TopicAlias(crate::ast::TopicAlias::Odom)),
            Some("topic_messages")
        );
    }

    #[test]
    fn table_scoped_resolution_prefers_matching_table() {
        let mut reg = FieldRegistry::new();
        // Register the same field name for two tables.
        reg.register(FieldDef {
            name: "robot_id".into(),
            source_table: "topic_messages".into(),
            column: "robot_id".into(),
            storage_unit: None,
            is_map_access: false,
            map_column: None,
            map_key: None,
            metric_filter: None,
        });
        reg.register(FieldDef {
            name: "robot_id".into(),
            source_table: "otel_traces".into(),
            column: "resource_attributes".into(),
            storage_unit: None,
            is_map_access: true,
            map_column: Some("resource_attributes".into()),
            map_key: Some("robot.id".into()),
            metric_filter: None,
        });
        let for_traces = reg.resolve_for_table("robot_id", "otel_traces").unwrap();
        assert!(for_traces.is_map_access);
        let for_topics = reg.resolve_for_table("robot_id", "topic_messages").unwrap();
        assert!(!for_topics.is_map_access);
        // resolve() returns first registered (topic_messages)
        let fallback = reg.resolve("robot_id").unwrap();
        assert_eq!(fallback.source_table, "topic_messages");
    }

    #[test]
    fn resolve_for_table_falls_back_when_no_table_match() {
        let mut reg = FieldRegistry::new();
        reg.register(FieldDef {
            name: "duration".into(),
            source_table: "otel_traces".into(),
            column: "Duration".into(),
            storage_unit: Some("ns".into()),
            is_map_access: false,
            map_column: None,
            map_key: None,
            metric_filter: None,
        });
        // Unknown table falls back to the one entry.
        let def = reg
            .resolve_for_table("duration", "some_other_table")
            .unwrap();
        assert_eq!(def.column, "Duration");
    }
}
