//! SQL dialect abstraction — handles differences between PostgreSQL, SQLite, and MySQL.

use crate::error::ROSQLError;
use serde::{Deserialize, Serialize};

/// Supported SQL dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlDialect {
    PostgreSQL,
    SQLite,
    MySQL,
}

impl SqlDialect {
    /// Detect the SQL dialect from a connection string URL scheme.
    pub fn from_url(url: &str) -> Result<Self, ROSQLError> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(SqlDialect::PostgreSQL)
        } else if url.starts_with("sqlite:") {
            Ok(SqlDialect::SQLite)
        } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
            Ok(SqlDialect::MySQL)
        } else {
            Err(ROSQLError::DriverError {
                message: format!(
                    "unsupported connection string scheme: '{url}'. \
                     Expected postgres://, sqlite:, or mysql://"
                ),
            })
        }
    }

    /// Generate a JSON/JSONB field access expression.
    pub fn json_access(&self, column: &str, key: &str) -> String {
        match self {
            SqlDialect::PostgreSQL => format!("{column}->>'{key}'"),
            SqlDialect::SQLite => format!("json_extract({column}, '$.{key}')"),
            SqlDialect::MySQL => {
                format!("JSON_UNQUOTE(JSON_EXTRACT({column}, '$.{key}'))")
            }
        }
    }

    /// The expression for the current timestamp.
    pub fn now_expr(&self) -> &'static str {
        match self {
            SqlDialect::PostgreSQL => "NOW()",
            SqlDialect::SQLite => "datetime('now')",
            SqlDialect::MySQL => "NOW()",
        }
    }

    /// Generate a "now minus interval" expression.
    pub fn interval_ago(&self, amount: f64, unit: &str) -> String {
        let sql_unit = normalize_time_unit(unit);
        match self {
            SqlDialect::PostgreSQL => {
                format!("{} - INTERVAL '{} {}'", self.now_expr(), amount, sql_unit)
            }
            SqlDialect::SQLite => {
                let seconds = time_unit_to_seconds(amount, unit);
                format!("datetime('now', '-{seconds} seconds')")
            }
            SqlDialect::MySQL => {
                format!("{} - INTERVAL {} {}", self.now_expr(), amount, sql_unit)
            }
        }
    }

    /// Generate a DATE_TRUNC expression for time bucketing.
    pub fn date_trunc(&self, unit: &str, column: &str) -> String {
        match self {
            SqlDialect::PostgreSQL => format!("DATE_TRUNC('{unit}', {column})"),
            SqlDialect::SQLite => {
                // Use strftime for SQLite time bucketing
                let fmt = match unit {
                    "minute" => "%Y-%m-%d %H:%M:00",
                    "hour" => "%Y-%m-%d %H:00:00",
                    "day" => "%Y-%m-%d 00:00:00",
                    _ => "%Y-%m-%d %H:%M:%S",
                };
                format!("strftime('{fmt}', {column})")
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
            SqlDialect::PostgreSQL => {
                format!("PERCENTILE_CONT({fraction}) WITHIN GROUP (ORDER BY {column})")
            }
            // SQLite and MySQL lack native PERCENTILE_CONT.
            // Use a subquery approximation.
            SqlDialect::SQLite | SqlDialect::MySQL => {
                format!(
                    "(SELECT {column} FROM (SELECT {column}, \
                     ROW_NUMBER() OVER (ORDER BY {column}) AS rn, \
                     COUNT(*) OVER () AS cnt \
                     FROM __TABLE__) sub \
                     WHERE rn = CAST(cnt * {fraction} AS INTEGER))"
                )
            }
        }
    }

    /// Generate a CORR() aggregate expression.
    pub fn corr_aggregate(&self, col_a: &str, col_b: &str) -> String {
        match self {
            SqlDialect::PostgreSQL => format!("CORR({col_a}, {col_b})"),
            // SQLite and MySQL lack CORR(); use the manual formula.
            SqlDialect::SQLite | SqlDialect::MySQL => {
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
            SqlDialect::PostgreSQL => format!("to_timestamp({value})"),
            SqlDialect::SQLite => format!("datetime({value}, 'unixepoch')"),
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

/// Normalize time unit words to SQL INTERVAL units.
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

/// Convert a time amount + unit to total seconds.
fn time_unit_to_seconds(amount: f64, unit: &str) -> f64 {
    match unit.to_lowercase().as_str() {
        "nanoseconds" | "nanosecond" | "ns" => amount * 1e-9,
        "microseconds" | "microsecond" | "us" => amount * 1e-6,
        "milliseconds" | "millisecond" | "ms" => amount * 1e-3,
        "seconds" | "second" | "s" => amount,
        "minutes" | "minute" | "min" => amount * 60.0,
        "hours" | "hour" | "h" => amount * 3600.0,
        "days" | "day" => amount * 86400.0,
        "weeks" | "week" => amount * 604800.0,
        _ => amount,
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
    fn detect_sqlite() {
        assert_eq!(
            SqlDialect::from_url("sqlite:./test.db").unwrap(),
            SqlDialect::SQLite
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
            "SpanAttributes->>'ros.node'"
        );
    }

    #[test]
    fn json_access_sqlite() {
        assert_eq!(
            SqlDialect::SQLite.json_access("SpanAttributes", "ros.node"),
            "json_extract(SpanAttributes, '$.ros.node')"
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
    fn interval_ago_sqlite() {
        let expr = SqlDialect::SQLite.interval_ago(1.0, "hours");
        assert_eq!(expr, "datetime('now', '-3600 seconds')");
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
