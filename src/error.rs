//! Error types for ROSQL parsing, unit conversion, and data availability.

use crate::span::SourceLocation;
use serde::{Deserialize, Serialize};

/// All errors produced by the ROSQL parser and unit system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error)]
pub enum ROSQLError {
    /// A syntax or grammar error encountered during parsing.
    #[error("{message} at {location}")]
    ParseError {
        message: String,
        location: SourceLocation,
        /// Optional "did you mean?" suggestion.
        suggestion: Option<String>,
    },

    /// A unit conversion or compatibility error.
    #[error("Unit error: {message}")]
    UnitError {
        message: String,
        location: Option<SourceLocation>,
    },

    /// A required data source (table) is not available in the backend.
    #[error("Data source unavailable: {data_source}. {message}")]
    DataSourceUnavailable {
        data_source: String,
        message: String,
    },

    /// A mutation keyword (INSERT, UPDATE, DELETE, DROP) was used.
    /// ROSQL is strictly read-only.
    #[error("{keyword} is not allowed — ROSQL is read-only (at {location})")]
    MutationRejected {
        keyword: String,
        location: SourceLocation,
    },

    /// A keyword reserved for a future version was used (ALERT WHEN, DEFINE SLO).
    #[error("{keyword} is reserved for a future ROSQL version (at {location})")]
    ReservedSyntax {
        keyword: String,
        location: SourceLocation,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display() {
        let err = ROSQLError::ParseError {
            message: "unexpected token".into(),
            location: SourceLocation {
                line: 1,
                column: 5,
                offset: 4,
            },
            suggestion: Some("SELECT".into()),
        };
        assert_eq!(err.to_string(), "unexpected token at line 1, column 5");
    }

    #[test]
    fn mutation_rejected_display() {
        let err = ROSQLError::MutationRejected {
            keyword: "INSERT".into(),
            location: SourceLocation {
                line: 1,
                column: 1,
                offset: 0,
            },
        };
        assert!(err.to_string().contains("INSERT"));
        assert!(err.to_string().contains("read-only"));
    }

    #[test]
    fn reserved_syntax_display() {
        let err = ROSQLError::ReservedSyntax {
            keyword: "ALERT WHEN".into(),
            location: SourceLocation {
                line: 1,
                column: 1,
                offset: 0,
            },
        };
        assert!(err.to_string().contains("ALERT WHEN"));
        assert!(err.to_string().contains("future"));
    }
}
