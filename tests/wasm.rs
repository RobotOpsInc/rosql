//! WASM integration tests — test the wasm-bindgen exports in a real WASM runtime.
//!
//! Run with: `wasm-pack test --headless --chrome --features wasm`
//! Or: `just test-wasm`

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn json_from(val: JsValue) -> serde_json::Value {
    serde_wasm_bindgen::from_value(val).expect("result is not valid JSON")
}

// ---------------------------------------------------------------------------
// parse() tests
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn wasm_parse_valid_not_null() {
    let result = rosql::wasm::parse("SELECT * FROM logs");
    assert!(!result.is_null());
    assert!(!result.is_undefined());
}

#[wasm_bindgen_test]
fn wasm_parse_valid_has_ok_true_and_ast() {
    let result = rosql::wasm::parse("FROM traces WHERE status = 'ERROR'");
    let json = json_from(result);
    assert_eq!(json["ok"], true, "expected ok=true for valid parse");
    assert!(json["ast"].is_object(), "expected ast object");
}

#[wasm_bindgen_test]
fn wasm_parse_invalid_not_null() {
    let result = rosql::wasm::parse("SELCT * FROM logs");
    assert!(!result.is_null());
    assert!(!result.is_undefined());
}

#[wasm_bindgen_test]
fn wasm_parse_invalid_has_ok_false_and_error() {
    let result = rosql::wasm::parse("SELCT * FROM logs");
    let json = json_from(result);
    assert_eq!(json["ok"], false, "expected ok=false for invalid parse");
    assert!(
        json["error"].is_object(),
        "expected error object for invalid parse"
    );
}

#[wasm_bindgen_test]
fn wasm_parse_mutation_not_null() {
    let result = rosql::wasm::parse("INSERT INTO logs VALUES (1)");
    assert!(!result.is_null());
    assert!(!result.is_undefined());
}

#[wasm_bindgen_test]
fn wasm_parse_mutation_returns_error_object() {
    let result = rosql::wasm::parse("INSERT INTO logs VALUES (1)");
    let json = json_from(result);
    assert_eq!(
        json["ok"], false,
        "expected ok=false for mutation (INSERT is not valid ROSQL)"
    );
    assert!(
        json["error"].is_object(),
        "expected error object for mutation"
    );
}

// ---------------------------------------------------------------------------
// validate() tests
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn wasm_validate_valid_not_null() {
    let result = rosql::wasm::validate("FROM traces WHERE status = 'ERROR'");
    assert!(!result.is_null());
    assert!(!result.is_undefined());
}

#[wasm_bindgen_test]
fn wasm_validate_valid_has_valid_true_and_empty_errors() {
    let result = rosql::wasm::validate("FROM traces WHERE status = 'ERROR'");
    let json = json_from(result);
    assert_eq!(json["valid"], true, "expected valid=true");
    assert_eq!(
        json["errors"].as_array().map(|a| a.len()).unwrap_or(1),
        0,
        "expected empty errors array"
    );
}

#[wasm_bindgen_test]
fn wasm_validate_invalid_not_null() {
    let result = rosql::wasm::validate("INSERT INTO logs");
    assert!(!result.is_null());
    assert!(!result.is_undefined());
}

#[wasm_bindgen_test]
fn wasm_validate_invalid_has_valid_false_and_errors() {
    let result = rosql::wasm::validate("INSERT INTO logs");
    let json = json_from(result);
    assert_eq!(json["valid"], false, "expected valid=false for mutation");
    let errors = json["errors"].as_array().expect("expected errors array");
    assert!(!errors.is_empty(), "expected at least one error");
}

// ---------------------------------------------------------------------------
// get_completions() tests
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn wasm_completions_after_from() {
    let result = rosql::wasm::get_completions("FROM ", 5);
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn wasm_completions_after_from_returns_array() {
    let result = rosql::wasm::get_completions("FROM ", 5);
    let json = json_from(result);
    assert!(
        json.is_array(),
        "expected completions to be a JSON array, got: {json}"
    );
}

#[wasm_bindgen_test]
fn wasm_completions_after_from_suggests_data_sources() {
    let result = rosql::wasm::get_completions("FROM ", 5);
    let json = json_from(result);
    let s = json.to_string();
    assert!(
        s.contains("traces") || s.contains("logs") || s.contains("metrics"),
        "expected data-source suggestions after FROM, got: {s}"
    );
}

#[wasm_bindgen_test]
fn wasm_completions_after_number() {
    let result = rosql::wasm::get_completions("FROM traces WHERE duration > 500", 32);
    assert!(!result.is_null());
}
