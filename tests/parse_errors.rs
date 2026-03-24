//! Tests for error cases — mutation rejection, reserved syntax, suggestions.

use rosql::{parse, ROSQLError};

#[test]
fn mutation_insert() {
    let errs = parse("INSERT INTO logs VALUES (1)").unwrap_err();
    assert!(
        matches!(&errs[0], ROSQLError::MutationRejected { keyword, .. } if keyword == "INSERT")
    );
}

#[test]
fn mutation_update() {
    let errs = parse("UPDATE logs SET x = 1").unwrap_err();
    assert!(
        matches!(&errs[0], ROSQLError::MutationRejected { keyword, .. } if keyword == "UPDATE")
    );
}

#[test]
fn mutation_delete() {
    let errs = parse("DELETE FROM logs").unwrap_err();
    assert!(
        matches!(&errs[0], ROSQLError::MutationRejected { keyword, .. } if keyword == "DELETE")
    );
}

#[test]
fn mutation_create() {
    let errs = parse("CREATE TABLE logs (id INT)").unwrap_err();
    assert!(
        matches!(&errs[0], ROSQLError::MutationRejected { keyword, .. } if keyword == "CREATE")
    );
}

#[test]
fn reserved_alert() {
    let errs = parse("ALERT WHEN cpu > 90").unwrap_err();
    assert!(matches!(&errs[0], ROSQLError::ReservedSyntax { keyword, .. } if keyword == "ALERT"));
}

#[test]
fn reserved_define() {
    let errs = parse("DEFINE SLO availability 99.9").unwrap_err();
    assert!(matches!(&errs[0], ROSQLError::ReservedSyntax { keyword, .. } if keyword == "DEFINE"));
}

#[test]
fn did_you_mean_selct() {
    let errs = parse("SELCT * FROM logs").unwrap_err();
    match &errs[0] {
        ROSQLError::ParseError { suggestion, .. } => {
            let s = suggestion.as_ref().expect("expected suggestion");
            assert!(
                s.contains("SELECT"),
                "suggestion should mention SELECT, got: {s}"
            );
        }
        other => panic!("expected ParseError, got {other:?}"),
    }
}

#[test]
fn did_you_mean_whre() {
    // "WHRE" at the start position triggers the suggestion logic
    let errs = parse("WHRE duration > 500").unwrap_err();
    match &errs[0] {
        ROSQLError::ParseError { suggestion, .. } => {
            let s = suggestion.as_ref().expect("expected suggestion");
            assert!(
                s.contains("WHERE"),
                "suggestion should mention WHERE, got: {s}"
            );
        }
        other => panic!("expected ParseError, got {other:?}"),
    }
}

#[test]
fn unknown_data_source() {
    let errs = parse("FROM nonexistent_source").unwrap_err();
    assert!(
        matches!(&errs[0], ROSQLError::ParseError { message, .. } if message.contains("unknown data source"))
    );
}

#[test]
fn error_has_location() {
    let errs = parse("SELECT * FROM logs WHERE").unwrap_err();
    match &errs[0] {
        ROSQLError::ParseError { location, .. } => {
            assert!(location.line >= 1);
            assert!(location.column >= 1);
        }
        other => panic!("expected ParseError, got {other:?}"),
    }
}
