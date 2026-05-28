//! AC1: build smoke. The crate compiles and the binary's `--version`
//! reports the crate version. Smoke-only at iter-1; iter-2 swaps the
//! assert for an `assert_cmd`-driven invocation.

#[test]
fn version_string_present() {
    assert!(!agentns_claude::VERSION.is_empty(), "VERSION must not be empty");
    assert!(
        agentns_claude::VERSION.starts_with("0.1"),
        "VERSION should track Cargo.toml; got {}",
        agentns_claude::VERSION
    );
}
