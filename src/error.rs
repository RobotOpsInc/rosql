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

    /// A keyword that is reserved but not supported in ROSQL was used (ALERT, DEFINE).
    #[error("{message} (at {location})")]
    ReservedSyntax {
        keyword: String,
        location: SourceLocation,
        message: String,
    },

    /// A feature that parses correctly but is not yet implemented in the compiler.
    #[error("Not implemented: {feature}. {message}")]
    NotImplemented { feature: String, message: String },

    /// A driver or connection error.
    #[error("Driver error: {message}")]
    DriverError { message: String },

    /// An error compiling the AST to SQL.
    #[error("Compilation error: {message}")]
    CompilationError { message: String },
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
            keyword: "ALERT".into(),
            location: SourceLocation {
                line: 1,
                column: 1,
                offset: 0,
            },
            message: "ALERT is a reserved keyword but is not supported in ROSQL.".into(),
        };
        assert!(err.to_string().contains("ALERT"));
        assert!(err.to_string().contains("reserved keyword"));
    }

    #[test]
    fn not_implemented_display() {
        let err = ROSQLError::NotImplemented {
            feature: "NODE_STATUS()".into(),
            message: "Requires heartbeat data.".into(),
        };
        assert!(err.to_string().contains("Not implemented: NODE_STATUS()"));
        assert!(err.to_string().contains("heartbeat"));
    }
}
