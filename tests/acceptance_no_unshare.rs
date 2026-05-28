//! AC9: `--no-unshare` fallback. On a stock kernel, the flag yields exit 0
//! with a stderr warning; without the flag, the launcher refuses to run
//! and prints a clear "kernel does not support agent namespaces" message.

#![allow(clippy::unwrap_used, clippy::panic)]

use assert_cmd::Command;

fn bin() -> Command {
    let mut c = Command::cargo_bin("agentns-claude").unwrap();
    // Suppress mock-mode so the real fallback path runs.
    c.env_remove("AGENTNS_SESSION_ID_OVERRIDE");
    c.env_remove("AGENTNS_MOCK_FILE");
    c
}

fn on_wintermute_kernel() -> bool {
    std::path::Path::new("/proc/self/agent_session").exists()
}

#[test]
fn no_unshare_exits_zero_with_warning() {
    if on_wintermute_kernel() {
        // On wintermute, the real unshare path runs and the --no-unshare
        // sentinel test loses its meaning. Skip — AC9 stock-kernel branch
        // is already in scope; live ACs (5-8) cover wintermute behavior.
        return;
    }
    let out = bin()
        .args(["--intent", "test", "--no-unshare", "--", "true"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "--no-unshare must exit 0 on stock kernel; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err = String::from_utf8(out.stderr).unwrap().to_lowercase();
    assert!(
        err.contains("no-unshare") || err.contains("stock") || err.contains("synthesized"),
        "expected stock-kernel warning on stderr: {err}"
    );
}

#[test]
fn missing_no_unshare_exits_nonzero_on_stock_kernel() {
    if on_wintermute_kernel() {
        return;
    }
    let out = bin()
        .args(["--intent", "test", "--", "true"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "stock kernel without --no-unshare must fail");
    let err = String::from_utf8(out.stderr).unwrap().to_lowercase();
    assert!(
        err.contains("kernel does not support agent namespaces"),
        "expected clear remediation message; got {err}"
    );
}
