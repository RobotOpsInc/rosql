//! WASM integration tests — test the wasm-bindgen exports in a real WASM runtime.
//!
//! Run with: `wasm-pack test --headless --chrome --features wasm`
//! Or: `just test-wasm`

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

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
fn wasm_parse_invalid_not_null() {
    let result = rosql::wasm::parse("SELCT * FROM logs");
    assert!(!result.is_null());
    assert!(!result.is_undefined());
}

#[wasm_bindgen_test]
fn wasm_parse_mutation_not_null() {
    let result = rosql::wasm::parse("INSERT INTO logs VALUES (1)");
    assert!(!result.is_null());
    assert!(!result.is_undefined());
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
fn wasm_validate_invalid_not_null() {
    let result = rosql::wasm::validate("INSERT INTO logs");
    assert!(!result.is_null());
    assert!(!result.is_undefined());
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
fn wasm_completions_after_number() {
    let result = rosql::wasm::get_completions("FROM traces WHERE duration > 500", 32);
    assert!(!result.is_null());
}
