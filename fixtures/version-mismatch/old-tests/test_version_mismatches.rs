/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use uniffi_bindgen::bindings::{kotlin, python, swift, RunScriptOptions};

// Each of of the test below follows this general pattern:
//   - Trigger a versioning mismatch somehow
//   - Run a script in the bindings language we're testing
//   - Test that the script fails
//   - Test that the expected error messages are in the script output

#[test]
fn test_api_contract_mismatch_py() {
    run_python_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_contract_mismatch_output),
        expect_success: false,
        crate_features: vec![],
        uniffi_contract_version_override: Some(0),
    });
}

#[test]
fn test_api_contract_mismatch_kt() {
    run_kotlin_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_contract_mismatch_output),
        expect_success: false,
        crate_features: vec![],
        uniffi_contract_version_override: Some(0),
    });
}

#[test]
fn test_api_contract_mismatch_swift() {
    run_swift_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_contract_mismatch_output),
        expect_success: false,
        crate_features: vec![],
        uniffi_contract_version_override: Some(0),
    });
}

#[test]
fn test_udl_api_mismatch_py() {
    run_python_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_api_mismatch_output),
        expect_success: false,
        crate_features: vec!["udl_v2".into()],
        uniffi_contract_version_override: None,
    });
}

#[test]
fn test_udl_api_mismatch_kt() {
    run_kotlin_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_api_mismatch_output),
        expect_success: false,
        crate_features: vec!["udl_v2".into()],
        uniffi_contract_version_override: None,
    });
}

#[test]
fn test_udl_api_mismatch_swift() {
    run_swift_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_api_mismatch_output),
        expect_success: false,
        crate_features: vec!["udl_v2".into()],
        uniffi_contract_version_override: None,
    });
}

#[test]
fn test_proc_macro_api_mismatch_py() {
    run_python_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_api_mismatch_output),
        expect_success: false,
        crate_features: vec!["proc_macro_v2".into()],
        uniffi_contract_version_override: None,
    });
}

#[test]
fn test_proc_macro_api_mismatch_kt() {
    run_kotlin_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_api_mismatch_output),
        expect_success: false,
        crate_features: vec!["proc_macro_v2".into()],
        uniffi_contract_version_override: None,
    });
}

#[test]
fn test_proc_macro_api_mismatch_swift() {
    run_swift_script(RunScriptOptions {
        show_compiler_messages: true,
        check_output: Some(check_api_mismatch_output),
        expect_success: false,
        crate_features: vec!["proc_macro_v2".into()],
        uniffi_contract_version_override: None,
    });
}

fn run_python_script(options: RunScriptOptions) {
    python::run_script(
        std::env!("CARGO_TARGET_TMPDIR"),
        "uniffi-fixture-version-mismatch",
        "tests/bindings/python_test.py",
        vec![],
        &options,
    )
    .unwrap()
}

fn run_kotlin_script(options: RunScriptOptions) {
    kotlin::run_script(
        std::env!("CARGO_TARGET_TMPDIR"),
        "uniffi-fixture-version-mismatch",
        "tests/bindings/kotlin_test.kts",
        vec![],
        &options,
    )
    .unwrap()
}

fn run_swift_script(options: RunScriptOptions) {
    swift::run_script(
        std::env!("CARGO_TARGET_TMPDIR"),
        "uniffi-fixture-version-mismatch",
        "tests/bindings/swift_test.swift",
        vec![],
        &options,
    )
    .unwrap()
}

fn check_contract_mismatch_output(stdout: String, stderr: String) {
    ensure_output(
        "UniFFI contract version mismatch: try cleaning and rebuilding your project",
        &stdout,
        &stderr,
    );
}

fn check_api_mismatch_output(stdout: String, stderr: String) {
    ensure_output(
        "UniFFI API checksum mismatch: try cleaning and rebuilding your project",
        &stdout,
        &stderr,
    );
}

fn ensure_output(output: &str, stdout: &str, stderr: &str) {
    if !stdout.contains(output) && !stderr.contains(output) {
        panic!(
            "\
Script output did not contain the expected error output:
{output}

Either fix the script or adjust the expected output in `test_version_mismatches.rs`

STDOUT:
{stdout}

STDERR:
{stderr}
"
        );
    }
}
