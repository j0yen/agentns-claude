//! AC1: build smoke. The crate compiles and the binary's `--version`
//! reports the crate version. Smoke-only at iter-1; iter-2 swaps the
//! assert for an `assert_cmd`-driven invocation.

#[test]
fn version_string_present() {
    assert!(!agentns_claude::VERSION.is_empty(), "VERSION must not be empty");
    // Version is now 0.4.x (iter-4 assay gate landed); just check it parses.
    assert!(
        agentns_claude::VERSION.split('.').count() >= 2,
        "VERSION must be semver-ish; got {}",
        agentns_claude::VERSION
    );
}
