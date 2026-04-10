//! SqlBackend — SQL driver using native database clients.
//!
//! Uses native drivers for each database rather than AnyPool, to get full
//! type support. DuckDB uses its own synchronous crate (not sqlx).
//! Feature-gated behind `--features postgres`, `mysql`, or `duckdb`.
//!
//! # Parquet backend
//!
//! When `--backend parquet` is selected, use [`SqlBackend::from_parquet`] instead
//! of [`SqlBackend::new`]. This opens an in-memory DuckDB instance and creates
//! views over `read_parquet()` globs in the expected directory layout:
//!
//! ```text
//! <url>/
//!   traces/          *.parquet  → otel_traces view
//!   logs/            *.parquet  → otel_logs view
//!   metrics/         *.parquet  → otel_metrics view
//!   topic_messages/  *.parquet  → topic_messages view
//!   mcap_metadata/   *.parquet  → mcap_metadata view
//! ```
//!
//! S3 URLs (`s3://...`) automatically load the DuckDB `httpfs` extension and
//! configure credentials from standard AWS environment variables.

use async_trait::async_trait;
use std::time::Instant;

use crate::ast::{OutputFormat, Query};
use crate::error::ROSQLError;

use super::compiler::compile;
use super::dialect::SqlDialect;
use super::field_registry::FieldRegistry;
use super::otel_registry::default_otel_registry;
use super::{
    BackendCapabilities, ColumnMeta, EnrichmentMeta, ExecOptions, ROSQLBackend, ROSQLResult,
    ResultMetadata,
};

#[cfg(any(feature = "postgres", feature = "mysql"))]
use sqlx::{Column, Row};

#[cfg(feature = "duckdb")]
use std::sync::{Arc, Mutex};

/// Internal enum wrapping native database connections.
enum Pool {
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
    #[cfg(feature = "mysql")]
    MySql(sqlx::MySqlPool),
    #[cfg(feature = "duckdb")]
    DuckDb(Arc<Mutex<duckdb::Connection>>),
}

/// A SQL backend using native database drivers.
///
/// Supports PostgreSQL, MySQL, and DuckDB. The dialect is auto-detected
/// from the connection string URL scheme.
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

    /// Open an in-memory DuckDB instance that reads Parquet files from a directory.
    ///
    /// `url` is either a local path (e.g., `/data/export` or `./telemetry/`) or an
    /// S3 URI (`s3://bucket/prefix`). The directory must follow the demo-agent
    /// output layout:
    ///
    /// ```text
    /// <url>/
    ///   traces/          *.parquet  → otel_traces view
    ///   logs/            *.parquet  → otel_logs view
    ///   metrics/         *.parquet  → otel_metrics view
    ///   topic_messages/  *.parquet  → topic_messages view
    ///   mcap_metadata/   *.parquet  → mcap_metadata view
    /// ```
    ///
    /// Views are created silently even if a subdirectory is absent — the capability
    /// probing step will then report those tables as unavailable.
    ///
    /// ## S3 credentials
    ///
    /// When `url` starts with `s3://`, the DuckDB `httpfs` extension is loaded and
    /// credentials are read from the following environment variables:
    ///
    /// | Variable | Description |
    /// |---|---|
    /// | `AWS_ACCESS_KEY_ID` | Static access key |
    /// | `AWS_SECRET_ACCESS_KEY` | Static secret key |
    /// | `AWS_REGION` / `AWS_DEFAULT_REGION` | AWS region (e.g. `us-east-1`) |
    /// | `AWS_PROFILE` | Named credentials profile |
    /// | `AWS_ENDPOINT_URL` | Override for S3-compatible storage (MinIO, Ceph) |
    #[cfg(feature = "duckdb")]
    pub async fn from_parquet(url: &str) -> Result<Self, ROSQLError> {
        let url = url.to_string();
        tokio::task::spawn_blocking(move || {
            Self::from_parquet_sync(&url)
        })
        .await
        .map_err(|e| ROSQLError::DriverError {
            message: format!("parquet backend task failed: {e}"),
        })?
    }

    /// Create a Parquet backend with explicit capability overrides (used in tests).
    #[cfg(feature = "duckdb")]
    pub async fn from_parquet_with_capabilities(
        url: &str,
        capabilities: BackendCapabilities,
    ) -> Result<Self, ROSQLError> {
        let url = url.to_string();
        tokio::task::spawn_blocking(move || {
            Self::from_parquet_sync_inner(&url, None, Some(capabilities))
        })
        .await
        .map_err(|e| ROSQLError::DriverError {
            message: format!("parquet backend task failed: {e}"),
        })?
    }

    #[cfg(feature = "duckdb")]
    fn from_parquet_sync(url: &str) -> Result<Self, ROSQLError> {
        Self::from_parquet_sync_inner(url, None, None)
    }

    #[cfg(feature = "duckdb")]
    fn from_parquet_sync_inner(
        url: &str,
        _reserved: Option<()>,
        explicit_capabilities: Option<BackendCapabilities>,
    ) -> Result<Self, ROSQLError> {
        use std::env;

        let conn = duckdb::Connection::open_in_memory().map_err(|e| ROSQLError::DriverError {
            message: format!("failed to open in-memory DuckDB: {e}"),
        })?;

        // Normalise: strip trailing slash so glob paths are consistent.
        let base = url.trim_end_matches('/');

        // For S3 URLs, load the httpfs extension and configure AWS credentials.
        if base.starts_with("s3://") {
            conn.execute_batch("INSTALL httpfs; LOAD httpfs;")
                .map_err(|e| ROSQLError::DriverError {
                    message: format!("failed to load DuckDB httpfs extension: {e}"),
                })?;

            // Configure credentials from standard AWS environment variables.
            // We set each individually so unset variables are simply skipped.
            let cred_vars: &[(&str, &str)] = &[
                ("AWS_ACCESS_KEY_ID",     "s3_access_key_id"),
                ("AWS_SECRET_ACCESS_KEY", "s3_secret_access_key"),
                ("AWS_ENDPOINT_URL",      "s3_endpoint"),
                ("AWS_PROFILE",           "s3_profile"),
            ];
            for (env_var, duckdb_key) in cred_vars {
                if let Ok(val) = env::var(env_var) {
                    let sql = format!("SET {duckdb_key}='{val}'");
                    let _ = conn.execute_batch(&sql); // best-effort
                }
            }

            // Region: prefer AWS_REGION, fall back to AWS_DEFAULT_REGION.
            let region = env::var("AWS_REGION")
                .or_else(|_| env::var("AWS_DEFAULT_REGION"))
                .unwrap_or_default();
            if !region.is_empty() {
                let _ = conn.execute_batch(&format!("SET s3_region='{region}'"));
            }
        }

        // Table → subdirectory mapping following the demo-agent output layout.
        let view_mappings: &[(&str, &str)] = &[
            ("otel_traces",    "traces"),
            ("otel_logs",      "logs"),
            ("otel_metrics",   "metrics"),
            ("topic_messages", "topic_messages"),
            ("mcap_metadata",  "mcap_metadata"),
        ];

        for (view_name, subdir) in view_mappings {
            let glob_path = format!("{base}/{subdir}/**/*.parquet");
            let sql = format!(
                "CREATE VIEW IF NOT EXISTS {view_name} AS SELECT * FROM read_parquet('{glob_path}')"
            );
            // Silently skip views whose subdirectory has no matching files —
            // probe_capabilities will report them as unavailable.
            let _ = conn.execute_batch(&sql);
        }

        let pool = Pool::DuckDb(Arc::new(Mutex::new(conn)));

        let capabilities = if let Some(caps) = explicit_capabilities {
            caps
        } else {
            probe_capabilities_sync(&pool)
        };

        Ok(Self {
            pool,
            schema: default_otel_registry(),
            capabilities,
            dialect: SqlDialect::DuckDB,
        })
    }
}

#[async_trait]
impl ROSQLBackend for SqlBackend {
    async fn execute(&self, query: &Query, opts: &ExecOptions) -> Result<ROSQLResult, ROSQLError> {
        let effective_default_limit = opts.default_limit.or(Some(100));
        let compile_result = compile(
            query,
            &self.schema,
            &self.dialect,
            &self.capabilities,
            effective_default_limit,
        )?;
        let compiled_sql = compile_result.sql;
        let default_limit_applied = compile_result.default_limit_applied;
        let format_hint = compile_result.format_hint;
        let visualization = compile_result.visualization;
        let warnings = compile_result.warnings;

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
                    default_limit_applied,
                    enrichment_metadata: vec![],
                    format_hint,
                    visualization,
                    warnings,
                },
            });
        }

        let start = Instant::now();

        let (columns, mut result_rows) = self.run_sql(&compiled_sql).await?;

        // Enforce max_rows cap if requested.
        if let Some(max) = opts.max_rows {
            result_rows.truncate(max as usize);
        }

        // Phase 2: execute enrichment queries and merge into primary rows.
        let mut enrichment_metadata = Vec::new();
        if !compile_result.enrichments.is_empty() {
            let enrichment_meta = self
                .execute_enrichments(&compile_result.enrichments, &columns, &mut result_rows)
                .await?;
            enrichment_metadata = enrichment_meta;
        }

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
                default_limit_applied,
                enrichment_metadata,
                format_hint,
                visualization,
                warnings,
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
// SqlBackend helper methods
// ---------------------------------------------------------------------------

impl SqlBackend {
    /// Execute a raw SQL string against the backend pool.
    async fn run_sql(
        &self,
        sql: &str,
    ) -> Result<(Vec<ColumnMeta>, Vec<Vec<serde_json::Value>>), ROSQLError> {
        match &self.pool {
            #[cfg(feature = "postgres")]
            Pool::Postgres(pool) => execute_pg(pool, sql).await,
            #[cfg(feature = "mysql")]
            Pool::MySql(pool) => execute_mysql(pool, sql).await,
            #[cfg(feature = "duckdb")]
            Pool::DuckDb(conn) => {
                let conn = Arc::clone(conn);
                let sql = sql.to_string();
                tokio::task::spawn_blocking(move || {
                    let guard = conn.lock().expect("duckdb mutex poisoned");
                    execute_duckdb(&guard, &sql)
                })
                .await
                .map_err(|e| ROSQLError::ExecutionError {
                    message: "DuckDB task join error.".into(),
                    data_source: "DuckDB".into(),
                    compiled_sql: None,
                    suggestion: Some(format!("Internal error: {e}. This is likely a ROSQL bug — report at github.com/RobotOpsInc/rosql/issues")),
                })?
            }
        }
    }

    /// Execute enrichment queries (phase 2) and merge results into primary rows
    /// as a nested `_enriched` JSON value per source.
    async fn execute_enrichments(
        &self,
        plans: &[super::compiler::EnrichmentPlan],
        primary_cols: &[ColumnMeta],
        primary_rows: &mut [Vec<serde_json::Value>],
    ) -> Result<Vec<EnrichmentMeta>, ROSQLError> {
        use serde_json::{json, Map, Value};

        // Find trace_id column index in primary result
        let tid_col_idx = primary_cols
            .iter()
            .position(|c| c.name.to_lowercase() == "trace_id");

        let mut meta = Vec::new();

        for plan in plans {
            // Collect join key values from primary rows
            let join_values: Vec<String> = if let Some(idx) = tid_col_idx {
                primary_rows
                    .iter()
                    .filter_map(|row| row.get(idx))
                    .filter_map(|v| match v {
                        Value::String(s) => Some(format!("'{}'", s.replace('\'', "''"))),
                        _ => None,
                    })
                    .collect()
            } else {
                vec![]
            };

            if join_values.is_empty() {
                meta.push(EnrichmentMeta {
                    source: plan.source_name.clone(),
                    count: 0,
                    truncated: false,
                });
                continue;
            }

            let in_clause = join_values.join(", ");
            // Use a window function to enforce per-primary-row limit
            let enrichment_sql = format!(
                "SELECT * FROM (\
                 SELECT *, ROW_NUMBER() OVER (PARTITION BY {jc} ORDER BY \"Timestamp\") AS _enrich_rn \
                 FROM {tbl} WHERE {jc} IN ({in_clause})\
                 ) _enrich_sub WHERE _enrich_rn <= {limit}",
                jc = plan.join_column,
                tbl = plan.table,
                limit = plan.limit
            );

            let (enrichment_cols, enrichment_rows) = match self.run_sql(&enrichment_sql).await {
                Ok(result) => result,
                Err(_) => {
                    // Best-effort: if enrichment fails, skip it
                    meta.push(EnrichmentMeta {
                        source: plan.source_name.clone(),
                        count: 0,
                        truncated: false,
                    });
                    continue;
                }
            };

            // Build a join-key → [rows] map
            let enrich_jc_idx = enrichment_cols
                .iter()
                .position(|c| c.name.to_lowercase() == plan.join_column.to_lowercase());

            let mut grouped: std::collections::HashMap<String, Vec<Map<String, Value>>> =
                std::collections::HashMap::new();
            let mut total_count = 0usize;
            let mut any_truncated = false;

            for row in &enrichment_rows {
                let key = enrich_jc_idx
                    .and_then(|idx| row.get(idx))
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default();

                // Build a JSON object for this enrichment row (skip the internal rn column)
                let obj: Map<String, Value> = enrichment_cols
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.name != "_enrich_rn")
                    .map(|(i, c)| {
                        let v = row.get(i).cloned().unwrap_or(Value::Null);
                        (c.name.clone(), v)
                    })
                    .collect();

                let bucket = grouped.entry(key).or_default();
                bucket.push(obj);
                total_count += 1;
            }

            // Check truncation: if any bucket has exactly `limit` rows, it may be truncated
            for bucket in grouped.values() {
                if bucket.len() as u64 >= plan.limit {
                    any_truncated = true;
                    break;
                }
            }

            // Merge into primary rows
            let source_key = plan.source_name.clone();
            let truncated_key = format!("{source_key}_truncated");
            let count_key = format!("{source_key}_count");

            if let Some(tid_idx) = tid_col_idx {
                for row in primary_rows.iter_mut() {
                    let tid = row
                        .get(tid_idx)
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();

                    let enrich_rows = grouped.get(&tid).cloned().unwrap_or_default();
                    let truncated = enrich_rows.len() as u64 >= plan.limit;
                    let count = enrich_rows.len();

                    // Find or create the _enriched column (last column)
                    // We append it as a new value; columns are updated separately below
                    let enriched_val = row.last_mut().and_then(|v| {
                        if let Value::Object(m) = v {
                            Some(m)
                        } else {
                            None
                        }
                    });

                    if let Some(m) = enriched_val {
                        // Add to existing _enriched object
                        m.insert(source_key.clone(), json!(enrich_rows));
                        m.insert(truncated_key.clone(), json!(truncated));
                        m.insert(count_key.clone(), json!(count));
                    } else {
                        // Create new _enriched object
                        let mut enriched_obj = Map::new();
                        enriched_obj.insert(source_key.clone(), json!(enrich_rows));
                        enriched_obj.insert(truncated_key.clone(), json!(truncated));
                        enriched_obj.insert(count_key.clone(), json!(count));
                        row.push(Value::Object(enriched_obj));
                    }
                }
            }

            meta.push(EnrichmentMeta {
                source: plan.source_name.clone(),
                count: total_count,
                truncated: any_truncated,
            });
        }

        // Add _enriched to columns if any enrichment was applied
        // (caller holds &mut to rows but not columns — we handle via metadata)

        Ok(meta)
    }
}

// ---------------------------------------------------------------------------
// PostgreSQL execution
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
fn pg_execution_error(e: sqlx::Error, sql: &str) -> ROSQLError {
    let raw = e.to_string();
    let (message, suggestion) = classify_db_error(&raw);
    ROSQLError::ExecutionError {
        message,
        data_source: "PostgreSQL".into(),
        compiled_sql: Some(sql.to_string()),
        suggestion,
    }
}

#[cfg(feature = "postgres")]
async fn execute_pg(
    pool: &sqlx::PgPool,
    sql: &str,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<serde_json::Value>>), ROSQLError> {
    let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(sql)
        .fetch_all(pool)
        .await
        .map_err(|e| pg_execution_error(e, sql))?;

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

#[cfg(feature = "postgres")]
fn pg_value_to_json(row: &sqlx::postgres::PgRow, i: usize) -> serde_json::Value {
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
    if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(i) {
        return serde_json::Value::String(v.to_rfc3339());
    }
    if let Ok(v) = row.try_get::<serde_json::Value, _>(i) {
        return v;
    }
    serde_json::Value::Null
}

// ---------------------------------------------------------------------------
// MySQL execution
// ---------------------------------------------------------------------------

#[cfg(feature = "mysql")]
fn mysql_execution_error(e: sqlx::Error, sql: &str) -> ROSQLError {
    let raw = e.to_string();
    let (message, suggestion) = classify_db_error(&raw);
    ROSQLError::ExecutionError {
        message,
        data_source: "MySQL".into(),
        compiled_sql: Some(sql.to_string()),
        suggestion,
    }
}

#[cfg(feature = "mysql")]
async fn execute_mysql(
    pool: &sqlx::MySqlPool,
    sql: &str,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<serde_json::Value>>), ROSQLError> {
    let rows: Vec<sqlx::mysql::MySqlRow> = sqlx::query(sql)
        .fetch_all(pool)
        .await
        .map_err(|e| mysql_execution_error(e, sql))?;

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
// DuckDB execution
// ---------------------------------------------------------------------------

#[cfg(feature = "duckdb")]
fn duckdb_execution_error(e: duckdb::Error, sql: &str) -> ROSQLError {
    let raw = e.to_string();
    let (message, suggestion) = classify_db_error(&raw);
    ROSQLError::ExecutionError {
        message,
        data_source: "DuckDB".into(),
        compiled_sql: Some(sql.to_string()),
        suggestion,
    }
}

#[cfg(feature = "duckdb")]
fn execute_duckdb(
    conn: &duckdb::Connection,
    sql: &str,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<serde_json::Value>>), ROSQLError> {
    // Execute a LIMIT 0 query to get column metadata without fetching all rows.
    // column_count() / column_name() require the statement to be executed first.
    let meta_sql = format!("SELECT * FROM ({sql}) AS __rosql_meta LIMIT 0");
    let mut meta = conn
        .prepare(&meta_sql)
        .map_err(|e| duckdb_execution_error(e, sql))?;
    // Execute and immediately drop Rows<'_> so the borrow on `meta` is released.
    let _ = meta.query([]).map_err(|e| duckdb_execution_error(e, sql))?;
    let col_count = meta.column_count();
    let columns: Vec<ColumnMeta> = (0..col_count)
        .map(|i| ColumnMeta {
            name: meta
                .column_name(i)
                .cloned()
                .unwrap_or_else(|_| "?".to_string()),
            data_type: "unknown".to_string(),
            unit: None,
        })
        .collect();

    // Execute the real query.
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| duckdb_execution_error(e, sql))?;
    let rows_iter = stmt
        .query_map([], |row| {
            Ok((0..col_count)
                .map(|i| duckdb_value_to_json(row, i))
                .collect::<Vec<_>>())
        })
        .map_err(|e| duckdb_execution_error(e, sql))?;

    let mut result_rows = Vec::new();
    for row in rows_iter {
        result_rows.push(row.map_err(|e| duckdb_execution_error(e, sql))?);
    }

    Ok((columns, result_rows))
}

#[cfg(feature = "duckdb")]
fn duckdb_value_to_json(row: &duckdb::Row<'_>, i: usize) -> serde_json::Value {
    if let Ok(v) = row.get::<_, i64>(i) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(v) = row.get::<_, f64>(i) {
        return serde_json::Number::from_f64(v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.get::<_, bool>(i) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(v) = row.get::<_, String>(i) {
        return serde_json::Value::String(v);
    }
    serde_json::Value::Null
}

// ---------------------------------------------------------------------------
// Error classification
// ---------------------------------------------------------------------------

/// Map a raw database error string to a human-readable message and an
/// actionable suggestion. Never leaks raw driver error text to the user.
fn classify_db_error(raw: &str) -> (String, Option<String>) {
    let lower = raw.to_lowercase();

    if lower.contains("relation") && lower.contains("does not exist") {
        return (
            "Table not found.".into(),
            Some(
                "Verify that telemetry data has been ingested. Run `SHOW TOPICS` or \
                 `SHOW DEPLOYMENTS` to check available data. See: rosql.org/docs/schema-reference"
                    .into(),
            ),
        );
    }
    if lower.contains("column") && lower.contains("does not exist") {
        return (
            "Column not found.".into(),
            Some(
                "Check your field names against the ROSQL field reference: \
                 rosql.org/docs/schema-reference"
                    .into(),
            ),
        );
    }
    if lower.contains("table") && lower.contains("does not exist") {
        return (
            "Table not found.".into(),
            Some(
                "Verify that telemetry data has been ingested and the correct schema profile is \
                 selected (--schema otel-postgres or otel-clickhouse). \
                 See: rosql.org/docs/schema-reference"
                    .into(),
            ),
        );
    }
    if lower.contains("connection refused")
        || lower.contains("could not connect")
        || lower.contains("connection timed out")
    {
        return (
            "Cannot reach database.".into(),
            Some("Verify your connection string and check that the database is running.".into()),
        );
    }
    if lower.contains("authentication failed") || lower.contains("password") {
        return (
            "Database authentication failed.".into(),
            Some(
                "Check the credentials in your connection string. The URL format is: \
                 postgresql://user:password@host:port/database"
                    .into(),
            ),
        );
    }
    if lower.contains("permission denied") || lower.contains("access denied") {
        return (
            "Permission denied.".into(),
            Some(
                "The database user lacks SELECT permission on the telemetry tables. \
                 Grant SELECT on otel_traces, otel_logs, otel_metrics."
                    .into(),
            ),
        );
    }
    if lower.contains("syntax error") {
        return (
            "SQL syntax error (this is a ROSQL compiler bug — please report it).".into(),
            Some("Report this at: github.com/RobotOpsInc/rosql/issues".into()),
        );
    }

    // Fallback: generic message without leaking raw driver text.
    (
        "Query execution failed.".into(),
        Some("Run with --verbose to see the compiled SQL. Report persistent issues at: github.com/RobotOpsInc/rosql/issues".into()),
    )
}

// ---------------------------------------------------------------------------
// Connection + capability probing
// ---------------------------------------------------------------------------

async fn connect(_url: &str, dialect: &SqlDialect) -> Result<Pool, ROSQLError> {
    match dialect {
        #[cfg(feature = "postgres")]
        SqlDialect::PostgreSQL => {
            let pool = sqlx::PgPool::connect(url)
                .await
                .map_err(|e| ROSQLError::DriverError {
                    message: format!("failed to connect: {e}"),
                })?;
            Ok(Pool::Postgres(pool))
        }
        #[cfg(feature = "mysql")]
        SqlDialect::MySQL => {
            let pool =
                sqlx::MySqlPool::connect(url)
                    .await
                    .map_err(|e| ROSQLError::DriverError {
                        message: format!("failed to connect: {e}"),
                    })?;
            Ok(Pool::MySql(pool))
        }
        #[allow(unreachable_patterns)]
        _ => Err(ROSQLError::DriverError {
            message: format!(
                "no driver compiled for dialect {dialect:?}. \
                 Enable the matching feature flag (postgres, mysql, duckdb)."
            ),
        }),
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

/// Synchronous capability probing used within spawn_blocking contexts (e.g. from_parquet).
#[cfg(feature = "duckdb")]
fn probe_capabilities_sync(pool: &Pool) -> BackendCapabilities {
    BackendCapabilities {
        topic_data: probe_table_sync(pool, "topic_messages"),
        recording_index: probe_table_sync(pool, "mcap_metadata"),
    }
}

#[cfg(feature = "duckdb")]
fn probe_table_sync(pool: &Pool, table: &str) -> bool {
    let sql = format!("SELECT 1 FROM {table} LIMIT 0");
    match pool {
        Pool::DuckDb(conn) => {
            let guard = conn.lock().expect("duckdb mutex poisoned");
            guard.prepare(&sql).is_ok()
        }
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

async fn probe_table(pool: &Pool, table: &str) -> bool {
    let sql = format!("SELECT 1 FROM {table} LIMIT 0");
    match pool {
        #[cfg(feature = "postgres")]
        Pool::Postgres(p) => sqlx::query(&sql).fetch_optional(p).await.is_ok(),
        #[cfg(feature = "mysql")]
        Pool::MySql(p) => sqlx::query(&sql).fetch_optional(p).await.is_ok(),
        #[cfg(feature = "duckdb")]
        Pool::DuckDb(conn) => {
            let conn = Arc::clone(conn);
            tokio::task::spawn_blocking(move || {
                let guard = conn.lock().expect("duckdb mutex poisoned");
                guard.prepare(&sql).is_ok()
            })
            .await
            .unwrap_or(false)
        }
    }
}
