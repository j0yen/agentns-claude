//! AC3: mock mode. `$AGENTNS_SESSION_ID_OVERRIDE` propagates into the child's
//! `AGENTNS_SESSION_ID`; a non-empty mock file beats the env var when both are
//! set.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::process::id;
use std::sync::atomic::{AtomicU32, Ordering};

use assert_cmd::Command;

static MOCK_COUNTER: AtomicU32 = AtomicU32::new(0);

fn unique_mock_path(tag: &str) -> PathBuf {
    let n = MOCK_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("agentns-mock-test-{}-{tag}-{n}", id()))
}

fn bin() -> Command {
    Command::cargo_bin("agentns-claude").unwrap()
}

#[test]
fn env_override_propagates_to_child() {
    let out = bin()
        .env("AGENTNS_SESSION_ID_OVERRIDE", "from-env-deadbeef-cafe-0001")
        .env_remove("AGENTNS_MOCK_FILE")
        .args(["--intent", "test", "--", "printenv", "AGENTNS_SESSION_ID"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "from-env-deadbeef-cafe-0001"
    );
}

#[test]
fn file_wins_over_env() {
    let path = unique_mock_path("file-wins");
    fs::write(&path, "from-file-deadbeef-cafe-0002\n").unwrap();
    let out = bin()
        .env("AGENTNS_SESSION_ID_OVERRIDE", "from-env-deadbeef-cafe-0001")
        .env("AGENTNS_MOCK_FILE", &path)
        .args(["--intent", "test", "--", "printenv", "AGENTNS_SESSION_ID"])
        .output()
        .unwrap();
    let _ = fs::remove_file(&path);
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "from-file-deadbeef-cafe-0002",
        "file value must win over env"
    );
}

#[test]
fn mock_announces_to_stderr() {
    let out = bin()
        .env("AGENTNS_SESSION_ID_OVERRIDE", "announce-test-0003")
        .env_remove("AGENTNS_MOCK_FILE")
        .args(["--intent", "test", "--", "true"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("MOCK MODE") && err.contains("announce-test-0003"),
        "mock announce missing from stderr: {err}"
    );
}
