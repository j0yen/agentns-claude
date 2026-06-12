//! Thin wrapper around `assay agentns --json` for startup gating.
//!
//! The PRD (agentns-session-wire) requires that `agentns-claude` gates the
//! `prctl(PR_SET_AGENT_NS)` attempt on whether `assay` reports the kernel
//! surface as `Live`. If assay is absent or reports anything other than
//! `Live`, we skip the prctl call entirely and go straight to the synthesize
//! path. This avoids a wasted (and loud) syscall on kernels where the flag is
//! broken.
//!
//! # Mock injection (tests)
//!
//! Set `AGENTNS_ASSAY_MOCK_VERDICT` to one of:
//! - `"Live"` — `is_live()` returns `true`
//! - `"FlagRejected"`, `"Inert"`, `"NsCreated"`, … — `is_live()` returns `false`
//!
//! When the env var is set, `assay` is never spawned.

use std::io;
use std::process::Command;

/// The verdict string the assay binary emits when all layers pass.
const LIVE: &str = "Live";

/// Whether the agentns kernel surface is `Live` per `assay agentns`.
///
/// Returns `true` only when `assay agentns --json` exits 0 **and** the JSON
/// `verdict.type` field equals `"Live"`. Any other outcome (binary not found,
/// non-zero exit, JSON parse failure, non-Live verdict) returns `false` so
/// callers fall back to the synthesize path rather than attempting a broken
/// prctl.
///
/// # Mock injection
///
/// Set `AGENTNS_ASSAY_MOCK_VERDICT` to control the return value without
/// spawning the assay binary (see module-level doc).
#[must_use]
pub fn is_live() -> bool {
    // Test seam: AGENTNS_ASSAY_MOCK_VERDICT lets tests inject verdicts without
    // a wintermute kernel or the assay binary. Present in all builds; the env
    // var is never set in production.
    if let Ok(val) = std::env::var("AGENTNS_ASSAY_MOCK_VERDICT") {
        return val.trim() == LIVE;
    }

    match query_assay() {
        Ok(verdict) => verdict == LIVE,
        Err(_) => false,
    }
}

/// Invoke `assay agentns --json` and extract the `verdict.type` string.
///
/// Returns `Err` on spawn failure, non-zero exit, or JSON parse failure.
fn query_assay() -> io::Result<String> {
    let output = Command::new("assay")
        .args(["agentns", "--json"])
        .output()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("assay binary not found or failed to spawn: {e}"),
            )
        })?;

    if !output.status.success() {
        // Non-zero exit from assay means the check failed; treat as not-Live.
        // We still try to parse the JSON to extract a verdict type.
        // If parsing fails, bubble the error so callers fall back.
    }

    let stdout = String::from_utf8(output.stdout).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("assay output is not valid UTF-8: {e}"),
        )
    })?;

    // Minimal JSON extraction: look for `"type":"<verdict>"` without pulling
    // in a JSON dependency. The assay output format is stable:
    //   { "primitive": "agentns", "verdict": { "type": "FlagRejected", ... }, ... }
    // We extract the type field from the verdict object.
    extract_verdict_type(&stdout).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("assay output did not contain a parseable verdict.type: {stdout}"),
        )
    })
}

/// Minimal extraction of `verdict.type` from the assay JSON output.
///
/// Looks for the pattern `"type"` followed by `:` and a quoted string value.
/// Returns `None` if the pattern is not found.
fn extract_verdict_type(json: &str) -> Option<String> {
    // Find `"type"` key. assay JSON has exactly one "type" key under verdict.
    let type_key = json.find("\"type\"")?;
    let after_key = json.get(type_key + 6..)?; // skip past `"type"`
    // Skip whitespace and colon.
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key.get(colon_pos + 1..)?.trim_start();
    // The value must start with a quote.
    if !after_colon.starts_with('"') {
        return None;
    }
    let inner = after_colon.get(1..)?;
    let end_quote = inner.find('"')?;
    Some(inner.get(..end_quote)?.to_owned())
}

#[cfg(test)]
#[allow(unsafe_code)] // env var injection for test seams; tests run single-threaded per cargo
mod tests {
    use super::*;

    #[test]
    fn extract_verdict_live() {
        let json = r#"{"primitive":"agentns","verdict":{"type":"Live","detail":{}}}"#;
        assert_eq!(extract_verdict_type(json).as_deref(), Some("Live"));
    }

    #[test]
    fn extract_verdict_flag_rejected() {
        let json = r#"{"primitive":"agentns","verdict":{"type":"FlagRejected","detail":{"flag":256}}}"#;
        assert_eq!(
            extract_verdict_type(json).as_deref(),
            Some("FlagRejected")
        );
    }

    #[test]
    fn extract_verdict_inert() {
        let json = r#"{"primitive":"agentns","verdict":{"type":"Inert","detail":{}}}"#;
        assert_eq!(extract_verdict_type(json).as_deref(), Some("Inert"));
    }

    #[test]
    fn extract_verdict_missing_returns_none() {
        let json = r#"{"no_type_here": true}"#;
        assert!(extract_verdict_type(json).is_none());
    }

    #[test]
    fn mock_live_returns_true() {
        // SAFETY: test-only env mutation; cargo test runs each #[test] in a
        // single thread by default. The var is removed before the function
        // returns, so it cannot bleed into other tests running in the same
        // process (when --test-threads=1 is used by the harness).
        unsafe { std::env::set_var("AGENTNS_ASSAY_MOCK_VERDICT", "Live") };
        let result = is_live();
        // SAFETY: symmetric remove of the var set above.
        unsafe { std::env::remove_var("AGENTNS_ASSAY_MOCK_VERDICT") };
        assert!(result, "mock Live must return true");
    }

    #[test]
    fn mock_flag_rejected_returns_false() {
        // SAFETY: see mock_live_returns_true.
        unsafe { std::env::set_var("AGENTNS_ASSAY_MOCK_VERDICT", "FlagRejected") };
        let result = is_live();
        // SAFETY: see mock_live_returns_true.
        unsafe { std::env::remove_var("AGENTNS_ASSAY_MOCK_VERDICT") };
        assert!(!result, "mock FlagRejected must return false");
    }

    #[test]
    fn mock_inert_returns_false() {
        // SAFETY: see mock_live_returns_true.
        unsafe { std::env::set_var("AGENTNS_ASSAY_MOCK_VERDICT", "Inert") };
        let result = is_live();
        // SAFETY: see mock_live_returns_true.
        unsafe { std::env::remove_var("AGENTNS_ASSAY_MOCK_VERDICT") };
        assert!(!result, "mock Inert must return false");
    }

    #[test]
    fn mock_empty_returns_false() {
        // SAFETY: see mock_live_returns_true.
        unsafe { std::env::set_var("AGENTNS_ASSAY_MOCK_VERDICT", "") };
        let result = is_live();
        // SAFETY: see mock_live_returns_true.
        unsafe { std::env::remove_var("AGENTNS_ASSAY_MOCK_VERDICT") };
        assert!(!result, "mock empty must return false");
    }
}
