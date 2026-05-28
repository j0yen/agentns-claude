//! AC4: exec semantics — exit code is inherited from the wrapped command;
//! a non-existent command yields a clear error and a non-zero exit code.

#![allow(clippy::unwrap_used, clippy::panic)]

use assert_cmd::Command;

fn bin() -> Command {
    let mut c = Command::cargo_bin("agentns-claude").unwrap();
    // Force --no-unshare so exec semantics are testable on any kernel.
    c.env("AGENTNS_SESSION_ID_OVERRIDE", "exec-test-fixed-id");
    c
}

#[test]
fn echo_succeeds_with_payload() {
    let out = bin()
        .args(["--intent", "test", "--", "echo", "hi"])
        .output()
        .unwrap();
    assert!(out.status.success(), "echo must exit 0; stderr={}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8(out.stdout).unwrap().trim(), "hi");
}

#[test]
fn false_propagates_exit_one() {
    let out = bin()
        .args(["--intent", "test", "--", "false"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "exit code from `false` must propagate");
}

#[test]
fn missing_program_reports_clear_error() {
    let out = bin()
        .args([
            "--intent",
            "test",
            "--",
            "definitely-not-on-path-zzzzzz-7f3c1",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "missing program must be non-zero");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("exec") && err.contains("definitely-not-on-path-zzzzzz-7f3c1"),
        "stderr should name the missing program: got {err}"
    );
    // 127 is the conventional "command not found" code; we map io::NotFound to it.
    assert_eq!(out.status.code(), Some(127));
}
