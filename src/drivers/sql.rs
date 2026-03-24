//! SqlBackend — SQL driver using native sqlx database pools.
//!
//! Uses the native driver for each database (PgPool, MySqlPool).
//! rather than AnyPool, to get full type support.
//! Feature-gated behind `--features sql`.

use async_trait::async_trait;
use sqlx::{Column, Row};
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

/// Internal enum wrapping native database pools.
enum Pool {
    Postgres(sqlx::PgPool),
    MySql(sqlx::MySqlPool),
}

/// A SQL backend using native sqlx database drivers.
///
/// Works with PostgreSQL, MySQL, and SQLite. The dialect is auto-detected
/// from the connection string. Uses native drivers (not AnyPool) for
/// full type support including TIMESTAMPTZ, JSONB, arrays, etc.
pub struct SqlBackend {
    pool: Pool,
    schema: FieldRegistry,
    capabilities: BackendCapabilities,
    dialect: SqlDialect,
}

impl SqlBackend {
    /// Connect to a database and auto-detect dialect and capabilities.
    pub async fn new(url: &str) -> Result<Self, ROSQLError> {
        let dialect = SqlDialect::from_url(url)?;
        let pool = connect(url, &dialect).await?;
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
        let pool = connect(url, &dialect).await?;
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

        let (columns, result_rows) = match &self.pool {
            Pool::Postgres(pool) => execute_pg(pool, &compiled_sql).await?,
            Pool::MySql(pool) => execute_mysql(pool, &compiled_sql).await?,
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;
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

// ---------------------------------------------------------------------------
// Native driver execution
// ---------------------------------------------------------------------------

async fn execute_pg(
    pool: &sqlx::PgPool,
    sql: &str,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<serde_json::Value>>), ROSQLError> {
    let rows: Vec<sqlx::postgres::PgRow> =
        sqlx::query(sql)
            .fetch_all(pool)
            .await
            .map_err(|e| ROSQLError::DriverError {
                message: format!("query execution failed: {e}"),
            })?;

    let columns: Vec<ColumnMeta> = if let Some(first) = rows.first() {
        first
            .columns()
            .iter()
            .map(|c| ColumnMeta {
                name: c.name().to_string(),
                data_type: c.type_info().to_string(),
                unit: None,
            })
            .collect()
    } else {
        Vec::new()
    };

    let result_rows: Vec<Vec<serde_json::Value>> = rows
        .iter()
        .map(|row| {
            row.columns()
                .iter()
                .enumerate()
                .map(|(i, _)| pg_value_to_json(row, i))
                .collect()
        })
        .collect();

    Ok((columns, result_rows))
}

fn pg_value_to_json(row: &sqlx::postgres::PgRow, i: usize) -> serde_json::Value {
    // Try types in order: i64, f64, bool, String, then fallback
    if let Ok(v) = row.try_get::<i64, _>(i) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(v) = row.try_get::<f64, _>(i) {
        return serde_json::Number::from_f64(v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<bool, _>(i) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(v) = row.try_get::<String, _>(i) {
        return serde_json::Value::String(v);
    }
    // PostgreSQL-specific: try chrono types
    if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(i) {
        return serde_json::Value::String(v.to_rfc3339());
    }
    if let Ok(v) = row.try_get::<serde_json::Value, _>(i) {
        return v;
    }
    serde_json::Value::Null
}

async fn execute_mysql(
    pool: &sqlx::MySqlPool,
    sql: &str,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<serde_json::Value>>), ROSQLError> {
    let rows: Vec<sqlx::mysql::MySqlRow> =
        sqlx::query(sql)
            .fetch_all(pool)
            .await
            .map_err(|e| ROSQLError::DriverError {
                message: format!("query execution failed: {e}"),
            })?;

    let columns: Vec<ColumnMeta> = if let Some(first) = rows.first() {
        first
            .columns()
            .iter()
            .map(|c| ColumnMeta {
                name: c.name().to_string(),
                data_type: c.type_info().to_string(),
                unit: None,
            })
            .collect()
    } else {
        Vec::new()
    };

    let result_rows: Vec<Vec<serde_json::Value>> = rows
        .iter()
        .map(|row| {
            row.columns()
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    row.try_get::<String, _>(i)
                        .map(serde_json::Value::String)
                        .or_else(|_| {
                            row.try_get::<i64, _>(i)
                                .map(|n| serde_json::Value::Number(n.into()))
                        })
                        .or_else(|_| {
                            row.try_get::<f64, _>(i).map(|f| {
                                serde_json::Number::from_f64(f)
                                    .map(serde_json::Value::Number)
                                    .unwrap_or(serde_json::Value::Null)
                            })
                        })
                        .unwrap_or(serde_json::Value::Null)
                })
                .collect()
        })
        .collect();

    Ok((columns, result_rows))
}

// ---------------------------------------------------------------------------
// Connection + capability probing
// ---------------------------------------------------------------------------

async fn connect(url: &str, dialect: &SqlDialect) -> Result<Pool, ROSQLError> {
    match dialect {
        SqlDialect::PostgreSQL => {
            let pool = sqlx::PgPool::connect(url)
                .await
                .map_err(|e| ROSQLError::DriverError {
                    message: format!("failed to connect: {e}"),
                })?;
            Ok(Pool::Postgres(pool))
        }
        SqlDialect::MySQL => {
            let pool =
                sqlx::MySqlPool::connect(url)
                    .await
                    .map_err(|e| ROSQLError::DriverError {
                        message: format!("failed to connect: {e}"),
                    })?;
            Ok(Pool::MySql(pool))
        }
    }
}

async fn probe_capabilities(pool: &Pool) -> BackendCapabilities {
    let topic_data = probe_table(pool, "topic_messages").await;
    let recording_index = probe_table(pool, "mcap_metadata").await;

    BackendCapabilities {
        topic_data,
        recording_index,
    }
}

async fn probe_table(pool: &Pool, table: &str) -> bool {
    let sql = format!("SELECT 1 FROM {table} LIMIT 0");
    match pool {
        Pool::Postgres(p) => sqlx::query(&sql).fetch_optional(p).await.is_ok(),
        Pool::Sqlite(p) => sqlx::query(&sql).fetch_optional(p).await.is_ok(),
        Pool::MySql(p) => sqlx::query(&sql).fetch_optional(p).await.is_ok(),
    }
}
