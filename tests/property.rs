//! Property-based tests using proptest.
//!
//! These test invariants that should hold for all inputs:
//! - Unit conversion never panics and produces finite values
//! - Parser never panics on arbitrary input
//! - Parsed ASTs survive JSON roundtrip

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Unit conversion: never panics, always produces finite f64
// ---------------------------------------------------------------------------

fn all_unit_symbols() -> Vec<&'static str> {
    rosql::units::all_unit_symbols()
}

proptest! {
    #[test]
    fn unit_conversion_never_panics(
        value in prop::num::f64::ANY,
        unit_idx in 0..65usize,
    ) {
        let symbols = all_unit_symbols();
        if unit_idx < symbols.len() {
            let symbol = symbols[unit_idx];
            // Should not panic — may return Ok or Err
            let _ = rosql::units::convert_to_si(value, symbol, None);
        }
    }

    #[test]
    fn unit_conversion_finite_for_normal_values(
        value in -1e12f64..1e12f64,
        unit_idx in 0..65usize,
    ) {
        let symbols = all_unit_symbols();
        if unit_idx < symbols.len() {
            let symbol = symbols[unit_idx];
            if let Ok((si_val, _)) = rosql::units::convert_to_si(value, symbol, None) {
                prop_assert!(si_val.is_finite(), "SI value should be finite for {} {}", value, symbol);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Parser: never panics on arbitrary input
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn parser_never_panics(input in "\\PC{0,200}") {
        // Should return Ok or Err, never panic
        let _ = rosql::parse(&input);
    }

    #[test]
    fn parser_never_panics_on_sql_like(
        keyword in prop::sample::select(vec![
            "SELECT", "FROM", "WHERE", "AND", "OR", "NOT",
            "SINCE", "BETWEEN", "LIMIT", "ORDER BY", "FACET",
            "MESSAGE JOURNEY", "HEALTH()", "ANOMALY", "TRACE",
        ]),
        field in "[a-z_]{1,10}",
        value in "[0-9]{1,5}",
    ) {
        let query = format!("{keyword} {field} > {value}");
        let _ = rosql::parse(&query);
    }
}

// ---------------------------------------------------------------------------
// Serialization roundtrip: parse → JSON → deserialize → equal
// ---------------------------------------------------------------------------

fn valid_queries() -> Vec<&'static str> {
    vec![
        "SELECT * FROM logs",
        "FROM traces WHERE status = 'ERROR'",
        "FROM logs SINCE 30 minutes ago",
        "FROM traces WHERE duration > 500 ms",
        "FROM odom LIMIT 5",
        "HEALTH() FOR ROBOT 'r1'",
        "MESSAGE JOURNEY FOR TRACE 'abc'",
        "ANOMALY(duration)",
        "TRACE 'abc123'",
        "SHOW RECORDING",
        "PATH DEVIATION FOR ROBOT 'r1'",
        "CORRELATE WITH metrics",
        "FROM traces | WHERE duration > 500 ms | FACET robot_id",
        "SELECT AVG(duration) AS avg_dur FROM traces",
        "FROM logs WHERE severity IN ('ERROR', 'WARN')",
        "FROM traces SINCE last action failure",
        "FROM traces BETWEEN '2026-01-01T00:00:00Z' AND '2026-12-31T23:59:59Z'",
        "FROM traces SINCE 1742306400",
        "FOR ROBOT 'r1' FROM logs",
        "FROM metrics COMPARE TO last week",
    ]
}

#[test]
fn serialization_roundtrip_all_valid_queries() {
    for query in valid_queries() {
        let ast = rosql::parse(query).unwrap_or_else(|e| {
            panic!("parse failed for '{query}': {e:?}");
        });
        let json = serde_json::to_string(&ast).unwrap_or_else(|e| {
            panic!("serialize failed for '{query}': {e}");
        });
        let roundtripped: rosql::Query = serde_json::from_str(&json).unwrap_or_else(|e| {
            panic!("deserialize failed for '{query}': {e}");
        });
        assert_eq!(ast, roundtripped, "roundtrip failed for '{query}'");
    }
}

proptest! {
    #[test]
    fn parsed_ast_survives_json_roundtrip(
        query_idx in 0..20usize,
    ) {
        let queries = valid_queries();
        if query_idx < queries.len() {
            let query = queries[query_idx];
            if let Ok(ast) = rosql::parse(query) {
                let json = serde_json::to_string(&ast).unwrap();
                let roundtripped: rosql::Query = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(ast, roundtripped);
            }
        }
    }
}
