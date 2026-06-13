//! Driver-parity conformance suite (ROB-297, AC-Q1).
//!
//! This test runs the OSS parser + dialect/registry conventions against the
//! shared, machine-readable fixture at `conformance/cases.json`. That fixture is
//! the *contract* that keeps the OSS rosql column/data-source conventions and the
//! go_backend ClickHouse planner from drifting:
//!
//!   * data-source -> table mappings (AC-P4: `tf`->tf_states, `joints`->joint_states,
//!     `node_graph`->node_graph_edges, ...)
//!   * the PascalCase ClickHouse column convention (AC-P2 case-sensitivity fix:
//!     `trace_id`->`TraceId`, `span_name`->`SpanName`, ...)
//!   * the inferred presentation-layer `format_hint` per query shape (incl. the
//!     `SHOW NODE GRAPH` -> NodeGraph and `MESSAGE FLOW` -> DirectedGraph shapes).
//!
//! The OSS rosql crate is a *parser* plus `src/drivers/` conventions — it does not
//! itself emit ClickHouse SQL (that's the go_backend planner, and a future OSS CH
//! driver is the separate ROB-224). So this suite asserts exactly what the OSS side
//! exposes: that the parser resolves each query to the right `DataSource`, that the
//! registry maps that source to the right table and the OTel columns to their
//! canonical PascalCase identifiers, and that format inference yields the right hint.
//!
//! go_backend consumes the SAME `conformance/cases.json` in a follow-up parity test
//! (a separate change): it loads the file, plans each `query`, and asserts the
//! generated SQL targets `expected_table` and references the PascalCase columns in
//! `column_cases` — so the two implementations are pinned to one source of truth.

use rosql::ast::{DataSource, PipelineStage, Query};
use rosql::drivers::field_registry::data_source_key;
use rosql::drivers::format_inference::infer_format;
use rosql::drivers::otel_registry::{otel_registry, SchemaProfile};
use rosql::parse;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Fixture {
    schema_profile: String,
    column_cases: Vec<ColumnCase>,
    query_cases: Vec<QueryCase>,
}

#[derive(Debug, Deserialize)]
struct ColumnCase {
    name: String,
    /// Single-table form: `table` + `expected_columns`.
    table: Option<String>,
    expected_columns: Option<HashMap<String, String>>,
    /// Multi-table form: `tables` = { table -> { field -> column } }.
    tables: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Debug, Deserialize)]
struct QueryCase {
    name: String,
    query: String,
    /// `None` for compound queries with no FROM source (e.g. TRACE, SHOW NODE GRAPH).
    data_source_key: Option<String>,
    expected_table: Option<String>,
    expected_format_hint: String,
}

fn load_fixture() -> Fixture {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/conformance/cases.json");
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read conformance fixture {path}: {e}"));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse conformance fixture {path}: {e}"))
}

fn profile_from_str(s: &str) -> SchemaProfile {
    match s {
        "OtelClickhouse" => SchemaProfile::OtelClickhouse,
        "OtelPostgres" => SchemaProfile::OtelPostgres,
        other => panic!("unknown schema_profile in fixture: {other:?}"),
    }
}

/// Extract the primary FROM data source from any query variant.
/// Compound queries (TRACE, SHOW NODE GRAPH, MESSAGE FLOW) have no FROM source.
fn primary_source(q: &Query) -> Option<DataSource> {
    match q {
        Query::Standard(sq) => Some(sq.data_source.clone()),
        Query::Pipeline(pq) => pq.stages.iter().find_map(|s| match s {
            PipelineStage::From(ds) => Some(ds.clone()),
            _ => None,
        }),
        Query::Compound(_) => None,
    }
}

/// Render the inferred `FormatHint` to the string the fixture uses (its Debug name).
fn hint_name(q: &Query) -> String {
    format!("{:?}", infer_format(q).0)
}

#[test]
fn conformance_query_cases() {
    let fx = load_fixture();
    let profile = profile_from_str(&fx.schema_profile);
    let reg = otel_registry(profile);

    let mut failures: Vec<String> = Vec::new();

    for case in &fx.query_cases {
        let ast = match parse(&case.query) {
            Ok(ast) => ast,
            Err(errs) => {
                failures.push(format!(
                    "[{}] query failed to parse: {:?}\n  query: {}",
                    case.name, errs, case.query
                ));
                continue;
            }
        };

        // 1. data_source_key (AC-P4 source identity)
        let actual_key = primary_source(&ast).map(|ds| data_source_key(&ds));
        if actual_key.as_deref() != case.data_source_key.as_deref() {
            failures.push(format!(
                "[{}] data_source_key mismatch: expected {:?}, got {:?}\n  query: {}",
                case.name, case.data_source_key, actual_key, case.query
            ));
        }

        // 2. data_source -> table mapping (AC-P4)
        let actual_table =
            primary_source(&ast).and_then(|ds| reg.table_name(&ds).map(str::to_string));
        if actual_table.as_deref() != case.expected_table.as_deref() {
            failures.push(format!(
                "[{}] table mapping mismatch: expected {:?}, got {:?}\n  query: {}",
                case.name, case.expected_table, actual_table, case.query
            ));
        }

        // 3. format_hint (presentation-layer shape; incl. DirectedGraph / NodeGraph)
        let actual_hint = hint_name(&ast);
        if actual_hint != case.expected_format_hint {
            failures.push(format!(
                "[{}] format_hint mismatch: expected {:?}, got {:?}\n  query: {}",
                case.name, case.expected_format_hint, actual_hint, case.query
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "conformance query-case failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn conformance_column_cases() {
    let fx = load_fixture();
    let profile = profile_from_str(&fx.schema_profile);
    let reg = otel_registry(profile);

    let mut failures: Vec<String> = Vec::new();

    // Flatten single-table and multi-table column cases into (table, field, expected_column).
    let mut checks: Vec<(String, String, String, String)> = Vec::new();
    for cc in &fx.column_cases {
        match (&cc.table, &cc.expected_columns, &cc.tables) {
            (Some(table), Some(cols), None) => {
                for (field, col) in cols {
                    checks.push((cc.name.clone(), table.clone(), field.clone(), col.clone()));
                }
            }
            (None, None, Some(tables)) => {
                for (table, cols) in tables {
                    for (field, col) in cols {
                        checks.push((cc.name.clone(), table.clone(), field.clone(), col.clone()));
                    }
                }
            }
            _ => panic!(
                "column_case '{}' must have either (table + expected_columns) or (tables)",
                cc.name
            ),
        }
    }

    for (case, table, field, expected_col) in checks {
        match reg.resolve_for_table(&field, &table) {
            Some(def) => {
                if def.source_table != table {
                    failures.push(format!(
                        "[{case}] field '{field}' resolved to table '{}' not '{table}'",
                        def.source_table
                    ));
                }
                if def.column != expected_col {
                    failures.push(format!(
                        "[{case}] {table}.{field} column mismatch: expected {:?}, got {:?}",
                        expected_col, def.column
                    ));
                }
            }
            None => failures.push(format!(
                "[{case}] field '{field}' not registered for table '{table}'"
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "conformance column-case failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Guard: every `DataSource` that maps to a table must be exercised by at least one
/// query case, so a newly-added source can't silently bypass the parity contract.
#[test]
fn conformance_covers_every_table_backed_source() {
    let fx = load_fixture();
    let covered: std::collections::HashSet<&str> = fx
        .query_cases
        .iter()
        .filter_map(|c| c.data_source_key.as_deref())
        .collect();

    // Every DataSource variant that resolves to a table, by its registry key.
    let all_sources = [
        DataSource::Logs,
        DataSource::SystemLogs,
        DataSource::Traces,
        DataSource::Metrics,
        DataSource::Diagnostics,
        DataSource::Topics,
        DataSource::Tf,
        DataSource::Heartbeats,
        DataSource::Recordings,
        DataSource::Events,
        DataSource::NodeGraph,
        DataSource::Joints,
    ];

    let reg = otel_registry(profile_from_str(&fx.schema_profile));
    let mut missing: Vec<String> = Vec::new();
    for ds in &all_sources {
        // Sanity: each source must resolve to a table in the registry.
        assert!(
            reg.table_name(ds).is_some(),
            "DataSource {ds:?} has no table mapping in the registry"
        );
        let key = data_source_key(ds);
        if !covered.contains(key.as_str()) {
            missing.push(key);
        }
    }

    assert!(
        missing.is_empty(),
        "these table-backed data sources are not covered by any conformance query case: {missing:?}"
    );
}
