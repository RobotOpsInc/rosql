//! CLI integration tests (issue #96, Section 6).
//!
//! Tests the `rosql` CLI binary for correct JSON output shapes across the
//! `compile`, `parse`, `validate`, and `completions` subcommands. These tests
//! require the `server` feature so the binary is built.
//!
//! Run with: `cargo test --test cli_integration --features server`
//!
//! Tests marked `#[ignore]` require a live database connection:
//!   `just test-examples`

#![cfg(feature = "server")]

use std::io::Write;
use std::process::{Command, Stdio};

fn rosql_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rosql"))
}

// ---------------------------------------------------------------------------
// parse subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_parse_valid_query_outputs_ok_true() {
    let output = rosql_bin()
        .args(["parse", "FROM traces WHERE status = 'ERROR'"])
        .output()
        .expect("failed to run rosql");

    assert!(output.status.success(), "expected exit 0");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert_eq!(json["ok"], true, "expected ok=true");
    assert!(json["ast"].is_object(), "expected ast to be an object");
}

#[test]
fn cli_parse_invalid_query_outputs_ok_false_and_exits_nonzero() {
    let output = rosql_bin()
        .args(["parse", "SELCT * FROM logs"])
        .output()
        .expect("failed to run rosql");

    assert!(!output.status.success(), "expected non-zero exit");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert_eq!(json["ok"], false, "expected ok=false");
    assert!(
        json["errors"].is_array(),
        "expected errors array in parse failure"
    );
}

#[test]
fn cli_parse_stdin() {
    let mut child = rosql_bin()
        .arg("parse")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn rosql");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"FROM logs SINCE 1 hour ago")
        .unwrap();

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success(), "expected exit 0 for stdin parse");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert_eq!(json["ok"], true);
}

// ---------------------------------------------------------------------------
// compile subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_compile_postgres_outputs_ok_sql_backend() {
    let output = rosql_bin()
        .args([
            "compile",
            "FROM traces WHERE status = 'ERROR'",
            "--backend",
            "postgres",
        ])
        .output()
        .expect("failed to run rosql");

    assert!(output.status.success(), "expected exit 0");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert_eq!(json["ok"], true);
    assert!(json["sql"].is_string(), "expected sql field");
    assert_eq!(json["backend"], "postgres");
}

#[test]
fn cli_compile_parquet_outputs_parquet_backend_field() {
    let duck_out = rosql_bin()
        .args([
            "compile",
            "FROM traces WHERE status = 'ERROR'",
            "--backend",
            "parquet",
        ])
        .output()
        .expect("failed to run rosql");

    assert!(duck_out.status.success());
    let duck_json: serde_json::Value = serde_json::from_slice(&duck_out.stdout).unwrap();
    assert_eq!(duck_json["ok"], true);
    assert_eq!(duck_json["backend"], "parquet");
    assert!(duck_json["sql"].is_string());
}

#[test]
fn cli_compile_timeseries_postgres_uses_date_bin() {
    let output = rosql_bin()
        .args([
            "compile",
            "SELECT COUNT(*) FROM traces TIMESERIES 5 min SINCE 1 hour ago",
            "--backend",
            "postgres",
        ])
        .output()
        .expect("failed to run rosql");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let sql = json["sql"].as_str().unwrap();
    assert!(
        sql.contains("date_bin") || sql.contains("DATE_BIN"),
        "expected PG date_bin for TIMESERIES, got:\n{sql}"
    );
}

#[test]
fn cli_compile_clickhouse_schema_uses_pascal_case() {
    let output = rosql_bin()
        .args([
            "compile",
            "FROM traces WHERE status = 'ERROR'",
            "--backend",
            "postgres",
            "--schema",
            "otel-clickhouse",
        ])
        .output()
        .expect("failed to run rosql");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let sql = json["sql"].as_str().unwrap();
    assert!(
        sql.contains("TraceId") || sql.contains("StatusCode") || sql.contains("SpanName"),
        "expected PascalCase columns in ClickHouse schema output:\n{sql}"
    );
}

#[test]
fn cli_compile_invalid_query_outputs_ok_false_and_exits_nonzero() {
    let output = rosql_bin()
        .args(["compile", "NOT A VALID QUERY @@@@", "--backend", "postgres"])
        .output()
        .expect("failed to run rosql");

    assert!(!output.status.success(), "expected non-zero exit");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert_eq!(json["ok"], false);
    // Parse failures emit { errors: [...] }; compile failures emit { error: "..." }
    assert!(
        json["errors"].is_array() || json["error"].is_string(),
        "expected error or errors field, got: {json}"
    );
}

#[test]
fn cli_compile_stdin() {
    let mut child = rosql_bin()
        .args(["compile", "--backend", "postgres"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn rosql");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"FROM traces SINCE 1 hour ago")
        .unwrap();

    let output = child.wait_with_output().expect("failed to wait");
    assert!(output.status.success(), "expected exit 0 for stdin compile");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
}

#[test]
fn cli_compile_file_flag() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "FROM traces WHERE duration > 500 ms").unwrap();

    let output = rosql_bin()
        .args([
            "compile",
            "--backend",
            "postgres",
            "--file",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run rosql");

    assert!(output.status.success(), "expected exit 0 for --file");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["ok"], true);
}

#[test]
fn cli_compile_unsupported_backend_bigquery_exits_nonzero_with_message() {
    let output = rosql_bin()
        .args(["compile", "FROM traces", "--backend", "bigquery"])
        .output()
        .expect("failed to run rosql");

    assert!(
        !output.status.success(),
        "expected non-zero exit for bigquery"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not yet supported") || stderr.contains("BigQuery"),
        "expected informative error message, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// validate subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_validate_valid_query_outputs_valid_true() {
    let output = rosql_bin()
        .args(["validate", "FROM traces WHERE status = 'ERROR'"])
        .output()
        .expect("failed to run rosql");

    assert!(output.status.success(), "expected exit 0");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert_eq!(json["valid"], true);
    assert!(json["errors"].is_array());
    assert_eq!(json["errors"].as_array().unwrap().len(), 0);
}

#[test]
fn cli_validate_mutation_outputs_valid_false_and_exits_nonzero() {
    let output = rosql_bin()
        .args(["validate", "INSERT INTO logs VALUES (1)"])
        .output()
        .expect("failed to run rosql");

    assert!(
        !output.status.success(),
        "expected non-zero exit for mutation"
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert_eq!(json["valid"], false);
    let errors = json["errors"].as_array().unwrap();
    assert!(!errors.is_empty(), "expected at least one error");
}

// ---------------------------------------------------------------------------
// completions subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_completions_after_from_returns_json_array() {
    let output = rosql_bin()
        .args(["completions", "FROM ", "5"])
        .output()
        .expect("failed to run rosql");

    assert!(output.status.success(), "expected exit 0");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is not valid JSON");
    assert!(json.is_array(), "completions output should be a JSON array");
    let items = json.as_array().unwrap();
    assert!(!items.is_empty(), "expected some completions after FROM");
}

#[test]
fn cli_completions_after_from_suggests_data_sources() {
    let output = rosql_bin()
        .args(["completions", "FROM ", "5"])
        .output()
        .expect("failed to run rosql");

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let completions_str = json.to_string();
    assert!(
        completions_str.contains("traces")
            || completions_str.contains("logs")
            || completions_str.contains("metrics"),
        "expected data-source completions after FROM, got: {completions_str}"
    );
}

// ---------------------------------------------------------------------------
// query / schema subcommands — require live DB (#[ignore])
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live DB: just test-examples"]
fn cli_query_missing_backend_exits_nonzero() {
    let output = rosql_bin()
        .args(["query", "FROM traces"])
        .output()
        .expect("failed to run rosql");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--backend") || stderr.contains("ROSQL_BACKEND"));
}

#[test]
#[ignore = "requires live DB: just test-examples"]
fn cli_query_missing_url_exits_nonzero() {
    let output = rosql_bin()
        .args(["query", "FROM traces", "--backend", "postgres"])
        .output()
        .expect("failed to run rosql");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--url") || stderr.contains("ROSQL_URL"));
}
