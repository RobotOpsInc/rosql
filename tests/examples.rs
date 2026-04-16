//! Integration tests that parse and compile every query in the example .rosql files.
//!
//! These ensure that all example queries shown in documentation remain valid
//! as the parser and compiler evolve. Runs in standard `cargo test` — no
//! database connection needed.

use rosql::drivers::compiler::compile;
use rosql::drivers::dialect::SqlDialect;
use rosql::drivers::otel_registry::default_otel_registry;
use rosql::drivers::BackendCapabilities;
use rosql::error::ROSQLError;

/// Extract individual queries from a .rosql file.
/// Each query is a contiguous block of non-empty, non-comment lines.
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

fn parse_and_compile(query: &str) {
    let ast = rosql::parse(query).unwrap_or_else(|errs| {
        panic!("Parse failed for query:\n  {query}\nErrors: {errs:?}");
    });

    let registry = default_otel_registry();
    let dialect = SqlDialect::PostgreSQL;
    let capabilities = BackendCapabilities {
        topic_data: true,
        recording_index: true,
    };

    compile(&ast, &registry, &dialect, &capabilities, None).unwrap_or_else(|err| {
        panic!("Compile failed for query:\n  {query}\nError: {err}");
    });
}

#[test]
fn example_basic_queries() {
    let content = include_str!("../examples/queries/basic.rosql");
    let queries = extract_queries(content);
    assert!(!queries.is_empty(), "no queries found in basic.rosql");
    for query in &queries {
        parse_and_compile(query);
    }
}

#[test]
fn example_compound_clause_queries() {
    let content = include_str!("../examples/queries/compound_clauses.rosql");
    let queries = extract_queries(content);
    assert!(
        !queries.is_empty(),
        "no queries found in compound_clauses.rosql"
    );
    let registry = default_otel_registry();
    let dialect = SqlDialect::PostgreSQL;
    let capabilities = BackendCapabilities {
        topic_data: true,
        recording_index: true,
    };
    for query in &queries {
        // Parse must always succeed
        let ast = rosql::parse(query).unwrap_or_else(|errs| {
            panic!("Parse failed for query:\n  {query}\nErrors: {errs:?}");
        });
        // Compilation may return NotImplemented for gated clauses — that's expected
        if let Err(err) = compile(&ast, &registry, &dialect, &capabilities, None) {
            assert!(
                matches!(err, ROSQLError::NotImplemented { .. }),
                "Unexpected compile error for query:\n  {query}\nError: {err}"
            );
        }
    }
}

#[test]
fn example_pipeline_queries() {
    let content = include_str!("../examples/queries/pipeline.rosql");
    let queries = extract_queries(content);
    assert!(!queries.is_empty(), "no queries found in pipeline.rosql");
    for query in &queries {
        parse_and_compile(query);
    }
}

#[test]
fn example_timeseries_queries() {
    let content = include_str!("../examples/queries/timeseries.rosql");
    let queries = extract_queries(content);
    assert!(!queries.is_empty(), "no queries found in timeseries.rosql");
    for query in &queries {
        parse_and_compile(query);
    }
}

#[test]
fn extract_queries_helper() {
    let content = "-- comment\nFROM logs\n\n-- another\nFROM traces WHERE x = 1\n";
    let queries = extract_queries(content);
    assert_eq!(queries, vec!["FROM logs", "FROM traces WHERE x = 1"]);
}

#[test]
fn extract_multiline_query() {
    let content = "FROM traces\n| WHERE duration > 500\n| FACET robot_id\n";
    let queries = extract_queries(content);
    assert_eq!(
        queries,
        vec!["FROM traces | WHERE duration > 500 | FACET robot_id"]
    );
}
