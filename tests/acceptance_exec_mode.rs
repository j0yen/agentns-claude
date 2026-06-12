//! AC6: AGENTNS_MODE is exported to the child environment.
//!
//! Extends the AC4 exec test to verify that `AGENTNS_MODE` appears in the
//! child's environment alongside `AGENTNS_SESSION_ID` and `AGENTNS_INTENT`.

#![allow(clippy::unwrap_used, clippy::panic)]

use assert_cmd::Command;

fn bin() -> Command {
    let mut c = Command::cargo_bin("agentns-claude").unwrap();
    // Force mock mode so tests run deterministically on any kernel.
    c.env("AGENTNS_SESSION_ID_OVERRIDE", "exec-mode-test-fixed-id-1234ab");
    c
}

#[test]
fn agentns_mode_present_in_child_env() {
    let out = bin()
        .args(["--intent", "test", "--", "env"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "command must succeed; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.lines().any(|l| l.starts_with("AGENTNS_MODE=")),
        "AGENTNS_MODE must be exported to child:\n{stdout}"
    );
}

#[test]
fn agentns_session_id_and_intent_still_present() {
    let out = bin()
        .args(["--intent", "build", "--", "env"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.lines().any(|l| l.starts_with("AGENTNS_SESSION_ID=")),
        "AGENTNS_SESSION_ID must still be exported:\n{stdout}"
    );
    assert!(
        stdout.lines().any(|l| l == "AGENTNS_INTENT=build"),
        "AGENTNS_INTENT=build must be in child env:\n{stdout}"
    );
}

#[test]
fn agentns_mode_is_non_empty() {
    let out = bin()
        .args(["--intent", "test", "--", "env"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let mode_line = stdout
        .lines()
        .find(|l| l.starts_with("AGENTNS_MODE="))
        .expect("AGENTNS_MODE must be in child env");
    let mode = mode_line.trim_start_matches("AGENTNS_MODE=");
    assert!(!mode.is_empty(), "AGENTNS_MODE must be non-empty");
}
