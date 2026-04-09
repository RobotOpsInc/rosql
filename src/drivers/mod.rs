//! Driver layer — `ROSQLBackend` trait and execution infrastructure.
//!
//! The driver layer sits between the parsed AST and a SQL database.
//! Each driver implements the `ROSQLBackend` trait and owns its own
//! AST → SQL dialect compilation.

pub mod compiler;
pub mod dialect;
pub mod field_registry;
pub mod otel_registry;

#[cfg(any(feature = "postgres", feature = "mysql", feature = "duckdb"))]
pub mod sql;

use crate::ast::{OutputFormat, Query};
use crate::error::ROSQLError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use field_registry::{FieldDef, FieldRegistry};

// ---------------------------------------------------------------------------
// ROSQLBackend trait
// ---------------------------------------------------------------------------

/// The open source execution interface for ROSQL.
///
/// Each driver implements this trait. The Robot Ops Go planner does NOT
/// implement this trait — it consumes the proto AST directly.
#[async_trait]
pub trait ROSQLBackend: Send + Sync {
    /// Execute a parsed ROSQL AST against this backend.
    async fn execute(&self, query: &Query, opts: &ExecOptions) -> Result<ROSQLResult, ROSQLError>;

    /// Return the field registry for this backend's schema.
    fn schema(&self) -> &FieldRegistry;

    /// Declare which optional data sources are present.
    fn capabilities(&self) -> &BackendCapabilities;
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Declares which optional data sources are present in the backend's schema.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BackendCapabilities {
    /// Whether `topic_messages` table is present.
    /// Required for `FROM topics`, `FROM odom`, `PATH DEVIATION`.
    pub topic_data: bool,

    /// Whether `mcap_metadata` table is present.
    /// Required for `SHOW RECORDING`, `FROM recordings`.
    pub recording_index: bool,
}

/// Options for query execution.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExecOptions {
    /// Query timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Maximum number of rows to return.
    pub max_rows: Option<u64>,
    /// If true, return the compiled SQL without executing it.
    pub dry_run: bool,
    /// Default LIMIT to apply when no explicit LIMIT is in the query.
    /// Defaults to Some(100) at execution time when not set.
    pub default_limit: Option<u64>,
}

/// The result of executing a ROSQL query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ROSQLResult {
    /// Column metadata.
    pub columns: Vec<ColumnMeta>,
    /// Row data — each row is a Vec of JSON values aligned with `columns`.
    pub rows: Vec<Vec<serde_json::Value>>,
    /// The output format of the result.
    pub format: OutputFormat,
    /// Execution metadata.
    pub metadata: ResultMetadata,
}

/// Metadata about a result column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnMeta {
    /// Column name.
    pub name: String,
    /// Type hint (e.g. "string", "number", "timestamp").
    pub data_type: String,
    /// Display unit, if applicable.
    pub unit: Option<String>,
}

/// Metadata about the query execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultMetadata {
    /// Number of rows returned.
    pub row_count: usize,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// The compiled SQL string (for debugging / dry_run).
    pub compiled_sql: String,
    /// Whether a default LIMIT of 100 was automatically applied to this query.
    pub default_limit_applied: bool,
    /// Per-enrichment metadata (populated when ENRICH WITH is used).
    pub enrichment_metadata: Vec<EnrichmentMeta>,
}

/// Metadata for one ENRICH WITH source in a result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnrichmentMeta {
    /// The enrichment data source name (e.g. "logs").
    pub source: String,
    /// Total enrichment rows returned across all primary rows.
    pub count: usize,
    /// True if any primary row hit the per-row enrichment limit.
    pub truncated: bool,
}
