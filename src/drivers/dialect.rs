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
    pub fn from_url(url: &str) -> Result<Self, ROSQLError> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(SqlDialect::PostgreSQL)
        } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
            Ok(SqlDialect::MySQL)
        } else if url.starts_with("duckdb://") || url.starts_with("md:") {
            Ok(SqlDialect::DuckDB)
        } else {
            Err(ROSQLError::DriverError {
                message: format!(
                    "unsupported connection string scheme: '{url}'. \
                     Expected postgres://, mysql://, or duckdb://"
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
                format!("{}::TIMESTAMP - INTERVAL '{} {}'", self.now_expr(), amount, sql_unit)
            }
            SqlDialect::MySQL => {
                format!("{} - INTERVAL {} {}", self.now_expr(), amount, sql_unit)
            }
        }
    }

    /// Generate a DATE_TRUNC expression for time bucketing.
    pub fn date_trunc(&self, unit: &str, column: &str) -> String {
        match self {
            SqlDialect::PostgreSQL | SqlDialect::DuckDB => format!("DATE_TRUNC('{unit}', {column})"),
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

    /// The timestamp column name used in the standard OTel schema.
    pub fn timestamp_column(&self) -> &'static str {
        "Timestamp"
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
