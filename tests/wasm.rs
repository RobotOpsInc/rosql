//! WASM integration tests — test the wasm-bindgen exports in a real WASM runtime.
//!
//! Run with: `wasm-pack test --headless --chrome --features wasm`
//! Or: `just test-wasm`

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------------------------------------------------------------------------
// parse() tests
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn wasm_parse_valid_returns_ast() {
    let result = rosql::wasm::parse("SELECT * FROM logs");
    assert!(!result.is_null());
    assert!(!result.is_undefined());

    let obj = js_sys::Object::from(result);
    let ok = js_sys::Reflect::get(&obj, &JsValue::from_str("ok")).unwrap();
    assert_eq!(ok, JsValue::TRUE);

    let ast = js_sys::Reflect::get(&obj, &JsValue::from_str("ast")).unwrap();
    assert!(!ast.is_null());
}

#[wasm_bindgen_test]
fn wasm_parse_invalid_returns_error() {
    let result = rosql::wasm::parse("SELCT * FROM logs");
    let obj = js_sys::Object::from(result);
    let ok = js_sys::Reflect::get(&obj, &JsValue::from_str("ok")).unwrap();
    assert_eq!(ok, JsValue::FALSE);

    let error = js_sys::Reflect::get(&obj, &JsValue::from_str("error")).unwrap();
    assert!(!error.is_null());

    let error_obj = js_sys::Object::from(error);
    let message = js_sys::Reflect::get(&error_obj, &JsValue::from_str("message")).unwrap();
    assert!(message.is_string());
}

#[wasm_bindgen_test]
fn wasm_parse_mutation_returns_error() {
    let result = rosql::wasm::parse("INSERT INTO logs VALUES (1)");
    let obj = js_sys::Object::from(result);
    let ok = js_sys::Reflect::get(&obj, &JsValue::from_str("ok")).unwrap();
    assert_eq!(ok, JsValue::FALSE);
}

// ---------------------------------------------------------------------------
// validate() tests
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn wasm_validate_valid() {
    let result = rosql::wasm::validate("FROM traces WHERE status = 'ERROR'");
    let obj = js_sys::Object::from(result);
    let valid = js_sys::Reflect::get(&obj, &JsValue::from_str("valid")).unwrap();
    assert_eq!(valid, JsValue::TRUE);

    let errors = js_sys::Reflect::get(&obj, &JsValue::from_str("errors")).unwrap();
    let errors_arr = js_sys::Array::from(&errors);
    assert_eq!(errors_arr.length(), 0);
}

#[wasm_bindgen_test]
fn wasm_validate_invalid() {
    let result = rosql::wasm::validate("INSERT INTO logs");
    let obj = js_sys::Object::from(result);
    let valid = js_sys::Reflect::get(&obj, &JsValue::from_str("valid")).unwrap();
    assert_eq!(valid, JsValue::FALSE);

    let errors = js_sys::Reflect::get(&obj, &JsValue::from_str("errors")).unwrap();
    let errors_arr = js_sys::Array::from(&errors);
    assert!(errors_arr.length() > 0);
}

// ---------------------------------------------------------------------------
// get_completions() tests
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn wasm_completions_after_from() {
    let result = rosql::wasm::get_completions("FROM ", 5);
    // Result is a JsValue containing an array of completion objects
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn wasm_completions_after_number() {
    let result = rosql::wasm::get_completions("FROM traces WHERE duration > 500", 32);
    assert!(!result.is_null());
}
