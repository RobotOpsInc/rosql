//! ROSQL — The query language for ROS2 telemetry data.
//!
//! This crate provides a lexer, parser, typed AST, and unit system for the
//! ROSQL language. It is the foundation for all ROSQL tooling: standalone
//! drivers, the WASM frontend module, and the gRPC parser sidecar.

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod units;

// Public convenience re-exports
pub use ast::{Query, ROSQLQuery};
pub use error::ROSQLError;
pub use parser::parse;
