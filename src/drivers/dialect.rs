//! SQL dialect abstraction — handles differences between PostgreSQL, MySQL, and DuckDB.

use crate::error::ROSQLError;
use serde::{Deserialize, Serialize};

/// Supported SQL dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlDialect {
    PostgreSQL,
    MySQL,
    DuckDB,
}

impl SqlDialect {
    /// Detect the SQL dialect from a connection string URL scheme.
    ///
    /// Supported schemes: `postgres://`, `postgresql://`, `mysql://`, `mariadb://`.
    ///
    /// Note: the Parquet backend (`--backend parquet`) does not use URL detection —
    /// it sets `SqlDialect::DuckDB` directly via `SqlBackend::from_parquet()`.
    pub fn from_url(url: &str) -> Result<Self, ROSQLError> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(SqlDialect::PostgreSQL)
        } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
            Ok(SqlDialect::MySQL)
        } else {
            Err(ROSQLError::DriverError {
                message: format!(
                    "unsupported connection string scheme: '{url}'. \
                     Expected postgres:// or mysql://. \
                     For Parquet files use --backend parquet --url <path>."
                ),
            })
        }
    }

    /// Quote an identifier (table or column name) to preserve case.
    /// PostgreSQL and DuckDB lowercase unquoted identifiers; MySQL does not.
    pub fn quote_ident(&self, ident: &str) -> String {
        match self {
            SqlDialect::PostgreSQL | SqlDialect::DuckDB => format!("\"{ident}\""),
            SqlDialect::MySQL => ident.to_string(),
        }
    }

    /// Generate a JSON field access expression.
    pub fn json_access(&self, column: &str, key: &str) -> String {
        let col = self.quote_ident(column);
        match self {
            SqlDialect::PostgreSQL | SqlDialect::DuckDB => format!("{col}->>'{key}'"),
            SqlDialect::MySQL => {
                format!("JSON_UNQUOTE(JSON_EXTRACT({col}, '$.{key}'))")
            }
        }
    }

    /// Generate a JSON field access that is guaranteed to return a VARCHAR/TEXT value.
    /// Use this when ordering or comparing extracted values, to avoid DuckDB type cast errors
    /// with JSON typed columns.
    pub fn json_access_text(&self, column: &str, key: &str) -> String {
        let col = self.quote_ident(column);
        match self {
            SqlDialect::DuckDB => format!("CAST({col}->>'{key}' AS VARCHAR)"),
            SqlDialect::PostgreSQL => format!("{col}->>'{key}'"),
            SqlDialect::MySQL => {
                format!("JSON_UNQUOTE(JSON_EXTRACT({col}, '$.{key}'))")
            }
        }
    }

    /// The expression for the current timestamp.
    pub fn now_expr(&self) -> &'static str {
        "NOW()"
    }

    /// Generate a "now minus interval" expression.
    pub fn interval_ago(&self, amount: f64, unit: &str) -> String {
        let sql_unit = normalize_time_unit(unit);
        match self {
            SqlDialect::PostgreSQL => {
                format!("{} - INTERVAL '{} {}'", self.now_expr(), amount, sql_unit)
            }
            // DuckDB's NOW() returns TIMESTAMPTZ; cast to TIMESTAMP for interval arithmetic.
            SqlDialect::DuckDB => {
                format!(
                    "{}::TIMESTAMP - INTERVAL '{} {}'",
                    self.now_expr(),
                    amount,
                    sql_unit
                )
            }
            SqlDialect::MySQL => {
                format!("{} - INTERVAL {} {}", self.now_expr(), amount, sql_unit)
            }
        }
    }

    /// Generate a DATE_TRUNC expression for time bucketing.
    pub fn date_trunc(&self, unit: &str, column: &str) -> String {
        match self {
            SqlDialect::PostgreSQL | SqlDialect::DuckDB => {
                format!("DATE_TRUNC('{unit}', {column})")
            }
            SqlDialect::MySQL => {
                let fmt = match unit {
                    "minute" => "%Y-%m-%d %H:%i:00",
                    "hour" => "%Y-%m-%d %H:00:00",
                    "day" => "%Y-%m-%d 00:00:00",
                    _ => "%Y-%m-%d %H:%i:%s",
                };
                format!("DATE_FORMAT({column}, '{fmt}')")
            }
        }
    }

    /// Generate a PERCENTILE_CONT expression.
    pub fn percentile_cont(&self, fraction: f64, column: &str) -> String {
        match self {
            SqlDialect::PostgreSQL | SqlDialect::DuckDB => {
                format!("PERCENTILE_CONT({fraction}) WITHIN GROUP (ORDER BY {column})")
            }
            SqlDialect::MySQL => {
                format!(
                    "(SELECT {column} FROM (SELECT {column}, \
                     ROW_NUMBER() OVER (ORDER BY {column}) AS rn, \
                     COUNT(*) OVER () AS cnt \
                     FROM __TABLE__) sub \
                     WHERE rn = CAST(cnt * {fraction} AS UNSIGNED))"
                )
            }
        }
    }

    /// Generate a CORR() aggregate expression.
    pub fn corr_aggregate(&self, col_a: &str, col_b: &str) -> String {
        match self {
            SqlDialect::PostgreSQL | SqlDialect::DuckDB => format!("CORR({col_a}, {col_b})"),
            SqlDialect::MySQL => {
                format!(
                    "(AVG({col_a} * {col_b}) - AVG({col_a}) * AVG({col_b})) / \
                     NULLIF(STDDEV({col_a}) * STDDEV({col_b}), 0)"
                )
            }
        }
    }

    /// Generate a timestamp conversion from Unix epoch seconds.
    pub fn from_epoch_seconds(&self, value: u64) -> String {
        match self {
            SqlDialect::PostgreSQL | SqlDialect::DuckDB => format!("to_timestamp({value})"),
            SqlDialect::MySQL => format!("FROM_UNIXTIME({value})"),
        }
    }

    /// Generate an approximate COUNT(DISTINCT) expression.
    pub fn approx_count_distinct(&self, col: &str) -> String {
        match self {
            SqlDialect::DuckDB => format!("approx_count_distinct({col})"),
            // PostgreSQL and MySQL fall back to exact COUNT(DISTINCT ...)
            SqlDialect::PostgreSQL | SqlDialect::MySQL => format!("COUNT(DISTINCT {col})"),
        }
    }

    /// Generate an approximate percentile expression.
    pub fn approx_percentile(&self, fraction: f64, col: &str) -> String {
        match self {
            SqlDialect::DuckDB => format!("approx_quantile({col}, {fraction})"),
            // PostgreSQL and MySQL use exact PERCENTILE_CONT
            SqlDialect::PostgreSQL | SqlDialect::MySQL => self.percentile_cont(fraction, col),
        }
    }

    /// Generate an expression for the difference in seconds between two timestamp expressions.
    pub fn timestamp_diff_seconds(&self, ts_a: &str, ts_b: &str) -> String {
        match self {
            SqlDialect::PostgreSQL => {
                format!("EXTRACT(EPOCH FROM ({ts_a} - {ts_b}))")
            }
            SqlDialect::DuckDB => {
                format!("EPOCH(({ts_a}::TIMESTAMP - {ts_b}::TIMESTAMP))")
            }
            SqlDialect::MySQL => {
                format!("TIMESTAMPDIFF(SECOND, {ts_b}, {ts_a})")
            }
        }
    }

    /// Generate a JSON array element access expression, e.g. for `fields['position[0]']`.
    ///
    /// Compiles `base->'field'->>N` for PostgreSQL/DuckDB, or MySQL's JSON_EXTRACT with array path.
    pub fn json_array_access(&self, column: &str, field: &str, index: usize) -> String {
        let col = self.quote_ident(column);
        match self {
            SqlDialect::PostgreSQL | SqlDialect::DuckDB => {
                format!("{col}->'{field}'->>{index}")
            }
            SqlDialect::MySQL => {
                format!("JSON_UNQUOTE(JSON_EXTRACT({col}, '$.{field}[{index}]'))")
            }
        }
    }

    /// The timestamp column name used in the standard OTel schema.
    pub fn timestamp_column(&self) -> &'static str {
        "Timestamp"
    }

    /// Generate a time-bucket expression for arbitrary intervals (TIMESERIES support).
    ///
    /// `seconds` is the bucket width in SI seconds (derived from UnitValue.si_value).
    /// Returns an expression that truncates a timestamp to the nearest bucket boundary.
    pub fn time_bucket(&self, seconds: f64, column: &str) -> String {
        let interval = seconds_to_interval_string(seconds);
        match self {
            SqlDialect::DuckDB => {
                // DuckDB native time_bucket function
                format!("time_bucket(INTERVAL '{interval}', {column}::TIMESTAMP)")
            }
            SqlDialect::PostgreSQL => {
                // date_bin available in PG14+; widely supported
                format!("date_bin('{interval}', {column}, TIMESTAMP '1970-01-01')")
            }
            SqlDialect::MySQL => {
                let secs = seconds.ceil() as u64;
                format!("FROM_UNIXTIME(UNIX_TIMESTAMP({column}) DIV {secs} * {secs})")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert an SI seconds value to a human-readable SQL interval string.
/// Examples: 300.0 → "5 minutes", 3600.0 → "1 hour", 86400.0 → "1 day"
fn seconds_to_interval_string(seconds: f64) -> String {
    // Choose the coarsest unit that divides evenly (up to floating-point tolerance)
    let s = seconds.round() as u64;
    if s % 86400 == 0 {
        let days = s / 86400;
        format!("{days} day{}", if days == 1 { "" } else { "s" })
    } else if s % 3600 == 0 {
        let hours = s / 3600;
        format!("{hours} hour{}", if hours == 1 { "" } else { "s" })
    } else if s % 60 == 0 {
        let minutes = s / 60;
        format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" })
    } else {
        format!("{s} second{}", if s == 1 { "" } else { "s" })
    }
}

fn normalize_time_unit(unit: &str) -> &str {
    match unit.to_lowercase().as_str() {
        "nanoseconds" | "nanosecond" | "ns" => "second",
        "microseconds" | "microsecond" | "us" => "second",
        "milliseconds" | "millisecond" | "ms" => "second",
        "seconds" | "second" | "s" => "second",
        "minutes" | "minute" | "min" => "minute",
        "hours" | "hour" | "h" => "hour",
        "days" | "day" => "day",
        "weeks" | "week" => "day",
        _ => "second",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_postgres() {
        assert_eq!(
            SqlDialect::from_url("postgresql://localhost/db").unwrap(),
            SqlDialect::PostgreSQL
        );
        assert_eq!(
            SqlDialect::from_url("postgres://localhost/db").unwrap(),
            SqlDialect::PostgreSQL
        );
    }

    #[test]
    fn detect_mysql() {
        assert_eq!(
            SqlDialect::from_url("mysql://localhost/db").unwrap(),
            SqlDialect::MySQL
        );
    }

    #[test]
    fn detect_unsupported() {
        assert!(SqlDialect::from_url("oracle://localhost/db").is_err());
    }

    #[test]
    fn duckdb_url_no_longer_recognized() {
        // duckdb:// URLs are no longer supported via from_url(). Users should
        // use --backend parquet --url <path> instead.
        let err = SqlDialect::from_url("duckdb://").unwrap_err();
        assert!(err.to_string().contains("parquet"), "error should mention parquet: {err}");
    }

    #[test]
    fn motherduck_url_no_longer_recognized() {
        let err = SqlDialect::from_url("md:my_db").unwrap_err();
        assert!(err.to_string().contains("parquet"), "error should mention parquet: {err}");
    }

    #[test]
    fn json_access_postgres() {
        assert_eq!(
            SqlDialect::PostgreSQL.json_access("SpanAttributes", "ros.node"),
            r#""SpanAttributes"->>'ros.node'"#
        );
    }

    #[test]
    fn json_access_mysql() {
        assert_eq!(
            SqlDialect::MySQL.json_access("SpanAttributes", "ros.node"),
            "JSON_UNQUOTE(JSON_EXTRACT(SpanAttributes, '$.ros.node'))"
        );
    }

    #[test]
    fn interval_ago_postgres() {
        let expr = SqlDialect::PostgreSQL.interval_ago(30.0, "minutes");
        assert_eq!(expr, "NOW() - INTERVAL '30 minute'");
    }

    #[test]
    fn interval_ago_duckdb() {
        // DuckDB NOW() returns TIMESTAMPTZ; must cast to TIMESTAMP for interval arithmetic.
        let expr = SqlDialect::DuckDB.interval_ago(1.0, "hour");
        assert_eq!(expr, "NOW()::TIMESTAMP - INTERVAL '1 hour'");
    }

    #[test]
    fn date_trunc_postgres() {
        assert_eq!(
            SqlDialect::PostgreSQL.date_trunc("minute", "Timestamp"),
            "DATE_TRUNC('minute', Timestamp)"
        );
    }

    #[test]
    fn from_epoch_postgres() {
        assert_eq!(
            SqlDialect::PostgreSQL.from_epoch_seconds(1742306400),
            "to_timestamp(1742306400)"
        );
    }
}
