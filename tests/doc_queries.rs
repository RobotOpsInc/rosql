//! Smoke tests for every copy-pasteable ROSQL query in user-facing documentation.
//!
//! The canonical query corpus is `examples/queries/doc_examples.rosql`. Every query
//! there must parse and compile against both PostgreSQL and DuckDB dialects.
//!
//! Compilation is allowed to return `ROSQLError::NotImplemented` for gated clauses
//! (currently: HEALTH(), CORRELATE, SHOW RECORDING). Any other error is a failure —
//! it means a doc query silently produces wrong results or fails outright for users.
//!
//! Run with: `cargo test --test doc_queries`

use rosql::drivers::compiler::compile;
use rosql::drivers::dialect::SqlDialect;
use rosql::drivers::otel_registry::default_otel_registry;
use rosql::drivers::BackendCapabilities;
use rosql::error::ROSQLError;

fn caps() -> BackendCapabilities {
    BackendCapabilities {
        topic_data: true,
        recording_index: true,
    }
}

fn extract_queries(content: &str) -> Vec<String> {
    let mut queries = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            if !current.trim().is_empty() {
                queries.push(current.trim().to_string());
                current.clear();
            }
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(trimmed);
    }
    if !current.trim().is_empty() {
        queries.push(current.trim().to_string());
    }
    queries
}

fn assert_doc_query(query: &str, dialect: &SqlDialect) {
    let ast = rosql::parse(query).unwrap_or_else(|errs| {
        panic!(
            "Parse failed for doc query ({:?}):\n  {query}\nErrors: {errs:?}",
            dialect
        );
    });

    let registry = default_otel_registry();
    if let Err(err) = compile(&ast, &registry, dialect, &caps(), None) {
        assert!(
            matches!(err, ROSQLError::NotImplemented { .. }),
            "Unexpected compile error for doc query ({dialect:?}):\n  {query}\nError: {err}"
        );
    }
}

#[test]
fn doc_examples_parse_and_compile_postgres() {
    let content = include_str!("../examples/queries/doc_examples.rosql");
    let queries = extract_queries(content);
    assert!(
        !queries.is_empty(),
        "no queries found in doc_examples.rosql"
    );
    for query in &queries {
        assert_doc_query(query, &SqlDialect::PostgreSQL);
    }
}

#[test]
fn doc_examples_parse_and_compile_duckdb() {
    let content = include_str!("../examples/queries/doc_examples.rosql");
    let queries = extract_queries(content);
    assert!(
        !queries.is_empty(),
        "no queries found in doc_examples.rosql"
    );
    for query in &queries {
        assert_doc_query(query, &SqlDialect::DuckDB);
    }
}
