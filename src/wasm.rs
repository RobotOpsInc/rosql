//! WASM exports for frontend parse/validate/compile/completions.
//!
//! Feature-gated behind `--features wasm`. Compiled to WASM via
//! `wasm-pack build --target web --features wasm`.

use wasm_bindgen::prelude::*;

use crate::completions;
use crate::drivers::{
    compiler, dialect::SqlDialect, otel_registry::default_otel_registry, BackendCapabilities,
};

/// Parse a ROSQL query string into a typed AST.
///
/// Returns a JSON object:
/// - `{ ok: true, ast: { ... } }` on success
/// - `{ ok: false, error: { message, line, column, suggestion } }` on failure
#[wasm_bindgen]
pub fn parse(query: &str) -> JsValue {
    match crate::parser::parse(query) {
        Ok(ast) => {
            let result = serde_json::json!({
                "ok": true,
                "ast": serde_json::to_value(&ast).unwrap_or(serde_json::Value::Null),
            });
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
        Err(errors) => {
            let error = &errors[0];
            let result = match error {
                crate::error::ROSQLError::ParseError {
                    message,
                    location,
                    suggestion,
                } => serde_json::json!({
                    "ok": false,
                    "error": {
                        "message": message,
                        "line": location.line,
                        "column": location.column,
                        "suggestion": suggestion,
                    }
                }),
                other => serde_json::json!({
                    "ok": false,
                    "error": {
                        "message": other.to_string(),
                    }
                }),
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
    }
}

/// Validate a ROSQL query string.
///
/// Returns a JSON object:
/// `{ valid: bool, errors: [...], warnings: [...] }`
///
/// Warnings are structured `{ code, message, suggestion }` objects surfaced
/// for non-fatal issues (e.g. `ANOMALY without FACET`) as well as
/// `NotImplemented` features like `HEALTH()` or `NODE_STATUS()`.
#[wasm_bindgen]
pub fn validate(query: &str) -> JsValue {
    match crate::parser::parse(query) {
        Ok(ast) => {
            // Second phase: attempt compilation to surface warnings.
            let registry = default_otel_registry();
            let dialect = SqlDialect::DuckDB;
            let capabilities = BackendCapabilities {
                topic_data: true,
                recording_index: true,
            };
            let warnings = match compiler::compile(&ast, &registry, &dialect, &capabilities, None) {
                Ok(cr) => cr
                    .warnings
                    .into_iter()
                    .map(|w| {
                        serde_json::json!({
                            "code": w.code,
                            "message": w.message,
                            "suggestion": w.suggestion,
                        })
                    })
                    .collect::<Vec<_>>(),
                Err(crate::error::ROSQLError::NotImplemented { feature, message }) => {
                    vec![serde_json::json!({
                        "code": "NOT_IMPLEMENTED",
                        "message": format!("{feature}: {message}"),
                        "suggestion": null,
                    })]
                }
                Err(_) => vec![],
            };
            let result = serde_json::json!({
                "valid": true,
                "errors": [],
                "warnings": warnings,
            });
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
        Err(errors) => {
            let error_list: Vec<serde_json::Value> = errors
                .iter()
                .map(|e| match e {
                    crate::error::ROSQLError::ParseError {
                        message,
                        location,
                        suggestion,
                    } => serde_json::json!({
                        "message": message,
                        "line": location.line,
                        "column": location.column,
                        "suggestion": suggestion,
                    }),
                    crate::error::ROSQLError::ReservedSyntax {
                        message, location, ..
                    } => serde_json::json!({
                        "message": message,
                        "line": location.line,
                        "column": location.column,
                    }),
                    other => serde_json::json!({
                        "message": other.to_string(),
                    }),
                })
                .collect();

            let result = serde_json::json!({
                "valid": false,
                "errors": error_list,
                "warnings": [],
            });
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
    }
}

/// Compile a ROSQL query string to SQL (DuckDB dialect).
///
/// Returns a JSON object:
/// - `{ ok: true, sql: "SELECT ...", default_limit_applied: bool, format_hint: "...", visualization: {...}, warnings: [...] }` on success
/// - `{ ok: false, error: { message, ... } }` on failure
#[wasm_bindgen]
pub fn compile(query: &str) -> JsValue {
    let registry = default_otel_registry();
    let dialect = SqlDialect::DuckDB;
    let capabilities = BackendCapabilities {
        topic_data: true,
        recording_index: true,
    };
    match crate::parser::parse(query) {
        Ok(ast) => match compiler::compile(&ast, &registry, &dialect, &capabilities, Some(100)) {
            Ok(cr) => {
                let warnings: Vec<serde_json::Value> = cr
                    .warnings
                    .iter()
                    .map(|w| {
                        serde_json::json!({
                            "code": w.code,
                            "message": w.message,
                            "suggestion": w.suggestion,
                        })
                    })
                    .collect();
                let enrichments: Vec<serde_json::Value> = cr
                    .enrichments
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "table": e.table,
                            "join_column": e.join_column,
                            "limit": e.limit,
                        })
                    })
                    .collect();
                let result = serde_json::json!({
                    "ok": true,
                    "sql": cr.sql,
                    "default_limit_applied": cr.default_limit_applied,
                    "format_hint": format!("{:?}", cr.format_hint),
                    "visualization": serde_json::to_value(&cr.visualization).unwrap_or(serde_json::Value::Null),
                    "warnings": warnings,
                    "enrichments": enrichments,
                });
                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
            }
            Err(e) => {
                let result =
                    serde_json::json!({ "ok": false, "error": { "message": e.to_string() } });
                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
            }
        },
        Err(errors) => {
            let error = &errors[0];
            let result = match error {
                crate::error::ROSQLError::ParseError {
                    message,
                    location,
                    suggestion,
                } => {
                    serde_json::json!({ "ok": false, "error": { "message": message, "line": location.line, "column": location.column, "suggestion": suggestion } })
                }
                other => {
                    serde_json::json!({ "ok": false, "error": { "message": other.to_string() } })
                }
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
    }
}

/// Get autocomplete suggestions at a cursor position.
///
/// Returns an array of `{ label, detail, kind }` objects.
#[wasm_bindgen]
pub fn get_completions(query: &str, cursor_pos: usize) -> JsValue {
    let completions = completions::get_completions(query, cursor_pos);
    serde_wasm_bindgen::to_value(&completions).unwrap_or(JsValue::NULL)
}
