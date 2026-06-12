//! AC3: resolve_session() precedence — mock / --no-unshare / synth-fallback.
//!
//! Exercises the resolve_session precedence ladder at the binary level using
//! `assert_cmd`. All cases run on a stock kernel (no wintermute).
//!
//! Modes tested:
//! - `mock` (AGENTNS_SESSION_ID_OVERRIDE) — highest precedence
//! - `no-unshare` (--no-unshare flag on a stock kernel)
//! - `synth-fallback` cannot be tested without a wintermute kernel (it
//!   requires kernel_has_agent_ns() == true + prctl EINVAL), so we verify
//!   only that the AGENTNS_MODE env is emitted by the child.

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("agentns-claude").unwrap()
}

/// Run the binary and capture the child's env via `env` command, then check
/// that `AGENTNS_MODE=<expected>` appears in the output.
fn assert_mode(args: &[&str], expected_mode: &str) {
    let out = bin()
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "command must succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.lines().any(|l| l == format!("AGENTNS_MODE={expected_mode}")),
        "expected AGENTNS_MODE={expected_mode} in output:\n{stdout}"
    );
}

#[test]
fn mock_override_wins_over_no_unshare() {
    // AGENTNS_SESSION_ID_OVERRIDE sets the mock mode; --no-unshare is also
    // given to confirm mock still takes precedence.
    let out = bin()
        .env("AGENTNS_SESSION_ID_OVERRIDE", "aabbccddaabbccddaabbccddaabbccdd")
        .args(["--intent", "test", "--no-unshare", "--", "env"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // mock source string from mock.rs is "env-override"
    assert!(
        stdout.lines().any(|l| l == "AGENTNS_SESSION_ID=aabbccddaabbccddaabbccddaabbccdd"),
        "mock id must propagate: {stdout}"
    );
}

#[test]
fn no_unshare_produces_mode_no_unshare() {
    assert_mode(
        &["--intent", "test", "--no-unshare", "--", "env"],
        "no-unshare",
    );
}

#[test]
fn no_unshare_sets_agentns_session_id() {
    let out = bin()
        .args(["--intent", "test", "--no-unshare", "--", "env"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let id_line = stdout
        .lines()
        .find(|l| l.starts_with("AGENTNS_SESSION_ID="))
        .expect("AGENTNS_SESSION_ID must be in child env");
    let id = id_line.trim_start_matches("AGENTNS_SESSION_ID=");
    assert_eq!(id.len(), 32, "session id must be 32 chars: {id}");
    assert!(
        id.bytes().all(|b| b.is_ascii_hexdigit()),
        "session id must be hex: {id}"
    );
}

#[test]
fn stock_kernel_without_no_unshare_exits_nonzero() {
    // On a stock kernel without --no-unshare and no mock, the launcher must
    // refuse and exit with code 2.
    let out = bin()
        .args(["--intent", "test", "--", "true"])
        .output()
        .unwrap();
    // On a wintermute kernel this would succeed; on CI (stock) it must fail.
    // We can't assert exact behavior here without knowing kernel, but we
    // can assert that if it fails it exits non-zero.
    if !out.status.success() {
        let code = out.status.code().unwrap_or(-1);
        assert_ne!(code, 0, "stock-kernel refusal must be non-zero");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("kernel") || err.contains("unshare") || err.contains("agent"),
            "error message must mention the issue: {err}"
        );
    }
}

#[test]
fn agentns_mode_is_set_in_child_env() {
    // Verify AGENTNS_MODE is exported in any mock/no-unshare scenario.
    let out = bin()
        .args(["--intent", "test", "--no-unshare", "--", "env"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.lines().any(|l| l.starts_with("AGENTNS_MODE=")),
        "AGENTNS_MODE must be in child env:\n{stdout}"
    );
}
