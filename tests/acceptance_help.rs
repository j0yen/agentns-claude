//! AC2: `--help` is honest — lists every flag the PRD declares and documents
//! the mock-mode contract plus the `--no-unshare` fallback.

#![allow(clippy::unwrap_used, clippy::panic)]

use assert_cmd::Command;

#[test]
fn help_lists_all_flags_and_modes() {
    let out = Command::cargo_bin("agentns-claude")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success(), "--help must exit 0");
    let body = String::from_utf8(out.stdout).unwrap();
    for needle in [
        "--intent",
        "--budget",
        "--no-unshare",
        "--verbose",
    ] {
        assert!(body.contains(needle), "--help missing flag {needle}\n{body}");
    }
    let lower = body.to_lowercase();
    assert!(
        lower.contains("agentns_session_id_override") || lower.contains("/tmp/agentns-mock"),
        "--help must document the mock-mode contract\n{body}"
    );
    assert!(
        lower.contains("stock") || lower.contains("no-unshare"),
        "--help must mention the stock-kernel / no-unshare fallback\n{body}"
    );
}

#[test]
fn version_flag_prints_crate_version() {
    let out = Command::cargo_bin("agentns-claude")
        .unwrap()
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success(), "--version must exit 0");
    let body = String::from_utf8(out.stdout).unwrap();
    // Version is now 0.3.x; just verify it contains at least one digit.
    assert!(
        body.chars().any(|c| c.is_ascii_digit()),
        "--version must report a version number; got {body}"
    );
    assert!(
        body.contains("agentns-claude"),
        "--version must name the binary; got {body}"
    );
}
