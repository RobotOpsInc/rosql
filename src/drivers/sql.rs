//! SqlBackend — generic SQL driver using `sqlx::AnyPool`.
//!
//! Supports PostgreSQL, MySQL, and SQLite via a single driver.
//! Feature-gated behind `--features sql`.

use async_trait::async_trait;
use sqlx::any::AnyRow;
use sqlx::{AnyPool, Column, Row};
use std::time::Instant;

use crate::ast::{OutputFormat, Query};
use crate::error::ROSQLError;

use super::compiler::compile;
use super::dialect::SqlDialect;
use super::field_registry::FieldRegistry;
use super::otel_registry::default_otel_registry;
use super::{
    BackendCapabilities, ColumnMeta, ExecOptions, ROSQLBackend, ROSQLResult, ResultMetadata,
};

/// A generic SQL backend using `sqlx::AnyPool`.
///
/// Works with PostgreSQL, MySQL, and SQLite. The dialect is auto-detected
/// from the connection string. Capabilities (topic_data, recording_index)
/// are auto-probed by checking for table existence.
pub struct SqlBackend {
    pool: AnyPool,
    schema: FieldRegistry,
    capabilities: BackendCapabilities,
    dialect: SqlDialect,
}

impl SqlBackend {
    /// Connect to a database and auto-detect dialect and capabilities.
    ///
    /// ```text
    /// postgresql://user:pass@host:5432/db
    /// sqlite:./telemetry.db
    /// mysql://user:pass@host:3306/db
    /// ```
    pub async fn new(url: &str) -> Result<Self, ROSQLError> {
        let dialect = SqlDialect::from_url(url)?;
        let pool: AnyPool = AnyPool::connect(url)
            .await
            .map_err(|e| ROSQLError::DriverError {
                message: format!("failed to connect: {e}"),
            })?;

        let capabilities = probe_capabilities(&pool).await;
        let schema = default_otel_registry();

        Ok(Self {
            pool,
            schema,
            capabilities,
            dialect,
        })
    }

    /// Connect with explicit capabilities (skip auto-probing).
    pub async fn new_with_capabilities(
        url: &str,
        capabilities: BackendCapabilities,
    ) -> Result<Self, ROSQLError> {
        let dialect = SqlDialect::from_url(url)?;
        let pool: AnyPool = AnyPool::connect(url)
            .await
            .map_err(|e| ROSQLError::DriverError {
                message: format!("failed to connect: {e}"),
            })?;

        let schema = default_otel_registry();

        Ok(Self {
            pool,
            schema,
            capabilities,
            dialect,
        })
    }
}

#[async_trait]
impl ROSQLBackend for SqlBackend {
    async fn execute(&self, query: &Query, opts: &ExecOptions) -> Result<ROSQLResult, ROSQLError> {
        let compiled_sql = compile(query, &self.schema, &self.dialect, &self.capabilities)?;

        // Dry run: return the SQL without executing
        if opts.dry_run {
            return Ok(ROSQLResult {
                columns: vec![ColumnMeta {
                    name: "sql".into(),
                    data_type: "string".into(),
                    unit: None,
                }],
                rows: vec![vec![serde_json::Value::String(compiled_sql.clone())]],
                format: OutputFormat::Table,
                metadata: ResultMetadata {
                    row_count: 1,
                    execution_time_ms: 0,
                    compiled_sql,
                },
            });
        }

        let start = Instant::now();

        let rows: Vec<AnyRow> = sqlx::query(&compiled_sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ROSQLError::DriverError {
                message: format!("query execution failed: {e}"),
            })?;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Build column metadata from the first row (if any)
        let columns: Vec<ColumnMeta> = if let Some(first_row) = rows.first() {
            first_row
                .columns()
                .iter()
                .map(|col| ColumnMeta {
                    name: col.name().to_string(),
                    data_type: col.type_info().to_string(),
                    unit: None,
                })
                .collect()
        } else {
            Vec::new()
        };

        // Convert rows to serde_json::Value
        let result_rows: Vec<Vec<serde_json::Value>> = rows
            .iter()
            .map(|row: &AnyRow| {
                row.columns()
                    .iter()
                    .enumerate()
                    .map(|(i, _col)| {
                        // Try to extract as various types; fall back to null
                        row.try_get::<String, _>(i)
                            .map(serde_json::Value::String)
                            .or_else(|_| {
                                row.try_get::<i64, _>(i)
                                    .map(|n: i64| serde_json::Value::Number(n.into()))
                            })
                            .or_else(|_| {
                                row.try_get::<f64, _>(i).map(|f| {
                                    serde_json::Number::from_f64(f)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null)
                                })
                            })
                            .or_else(|_| row.try_get::<bool, _>(i).map(serde_json::Value::Bool))
                            .unwrap_or(serde_json::Value::Null)
                    })
                    .collect()
            })
            .collect();

        let row_count = result_rows.len();

        Ok(ROSQLResult {
            columns,
            rows: result_rows,
            format: OutputFormat::Table,
            metadata: ResultMetadata {
                row_count,
                execution_time_ms,
                compiled_sql,
            },
        })
    }

    fn schema(&self) -> &FieldRegistry {
        &self.schema
    }

    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }
}

/// Probe the database for optional table existence.
async fn probe_capabilities(pool: &AnyPool) -> BackendCapabilities {
    let topic_data = probe_table(pool, "topic_messages").await;
    let recording_index = probe_table(pool, "mcap_metadata").await;

    BackendCapabilities {
        topic_data,
        recording_index,
    }
}

/// Check if a table exists by attempting a lightweight query.
async fn probe_table(pool: &AnyPool, table: &str) -> bool {
    let sql = format!("SELECT 1 FROM {table} LIMIT 0");
    sqlx::query(&sql).fetch_optional(pool).await.is_ok()
}
