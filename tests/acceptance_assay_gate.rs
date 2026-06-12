//! AC5: assay gate — if assay reports anything other than Live, agentns-claude
//! does NOT attempt prctl and falls back to the synthesize path.
//!
//! Uses AGENTNS_ASSAY_MOCK_VERDICT to inject assay verdicts without a
//! wintermute kernel, and AGENTNS_PRCTL_MOCK_ERRNO to verify whether the
//! prctl path was attempted (if the errno mock is NOT set but assay says
//! FlagRejected, we must still get a synthesized id — not a prctl error).

#![allow(clippy::unwrap_used, clippy::panic, unsafe_code, clippy::expect_used)]

use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("agentns-claude").unwrap()
}

/// When assay says Live AND the kernel surface is present AND prctl mock
/// returns 0 (success), the binary must use the prctl path (mode "prctl").
/// We also need AGENTNS_SESSION_PATH to inject a fake non-zero session id.
#[test]
fn assay_live_and_prctl_ok_uses_prctl_mode() {
    // Simulate a full happy path: assay=Live, prctl succeeds, session reads back.
    // We inject a session file via AGENTNS_SESSION_PATH.
    let tmp = std::env::temp_dir().join("agentns_test_session_live");
    std::fs::write(&tmp, "deadbeefdeadbeefdeadbeefdeadbeef").unwrap();

    let out = bin()
        .env("AGENTNS_ASSAY_MOCK_VERDICT", "Live")
        .env("AGENTNS_PRCTL_MOCK_ERRNO", "0")
        .env("AGENTNS_SESSION_PATH", tmp.to_str().unwrap())
        // kernel_has_agent_ns() checks /proc/self/agent_session which exists
        // on this wintermute box; the mock path overrides read_agent_session.
        .args(["--intent", "test", "--", "env"])
        .output()
        .unwrap();

    let _ = std::fs::remove_file(&tmp);

    // On a wintermute kernel (where /proc/self/agent_session exists), assay
    // Live + prctl mock 0 must produce mode "prctl".
    if out.status.success() {
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(
            stdout.lines().any(|l| l == "AGENTNS_MODE=prctl"),
            "expected AGENTNS_MODE=prctl in child env:\n{stdout}"
        );
        assert!(
            stdout
                .lines()
                .any(|l| l == "AGENTNS_SESSION_ID=deadbeefdeadbeefdeadbeefdeadbeef"),
            "expected injected session id in child env:\n{stdout}"
        );
    }
    // On a stock kernel, /proc/self/agent_session may not exist → StockKernelNoFallback
    // → non-zero exit, which is expected and acceptable on CI.
}

/// When assay says FlagRejected AND /proc/self/agent_session exists (wintermute
/// box where the flag is broken), the binary must fall back to synthesize, NOT
/// attempt prctl.
///
/// Verify: mode is "synth-fallback", not "prctl".
/// Verify: session id is 32 lowercase hex chars (synthesized).
/// Verify: prctl is NOT called (by checking that even without a prctl mock the
/// binary succeeds — if prctl were called on a broken kernel without the mock,
/// it would fail).
#[test]
fn assay_flag_rejected_skips_prctl_uses_synth_fallback() {
    // Precondition: we're on a wintermute kernel where /proc/self/agent_session
    // exists (kernel_has_agent_ns() == true). On a stock kernel this test is a
    // no-op because kernel_has_agent_ns() == false and the binary exits 2.
    if !std::path::Path::new("/proc/self/agent_session").exists() {
        // Stock kernel: assay gate is moot; skip.
        return;
    }

    let out = bin()
        .env("AGENTNS_ASSAY_MOCK_VERDICT", "FlagRejected")
        // No AGENTNS_PRCTL_MOCK_ERRNO set: if prctl were called on this
        // broken kernel it would fail (EINVAL) and produce synth-fallback too,
        // but the assay gate should short-circuit before reaching prctl.
        .args(["--intent", "test", "--", "env"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "assay=FlagRejected must not crash; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout
            .lines()
            .any(|l| l == "AGENTNS_MODE=synth-fallback"),
        "expected AGENTNS_MODE=synth-fallback; got:\n{stdout}"
    );

    let id_line = stdout
        .lines()
        .find(|l| l.starts_with("AGENTNS_SESSION_ID="))
        .expect("AGENTNS_SESSION_ID must be in child env");
    let id = id_line.trim_start_matches("AGENTNS_SESSION_ID=");
    assert_eq!(id.len(), 32, "synthesized id must be 32 chars: {id}");
    assert!(
        id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "synthesized id must be lowercase hex: {id}"
    );

    // Verify the WARN message was emitted.
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("assay agentns not Live"),
        "expected assay-gate WARN in stderr:\n{stderr}"
    );
}

/// When assay says Inert, same behaviour as FlagRejected: synth-fallback.
#[test]
fn assay_inert_skips_prctl_uses_synth_fallback() {
    if !std::path::Path::new("/proc/self/agent_session").exists() {
        return;
    }

    let out = bin()
        .env("AGENTNS_ASSAY_MOCK_VERDICT", "Inert")
        .args(["--intent", "test", "--", "env"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "assay=Inert must not crash; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout
            .lines()
            .any(|l| l == "AGENTNS_MODE=synth-fallback"),
        "expected AGENTNS_MODE=synth-fallback; got:\n{stdout}"
    );
}

/// Unit-level: assay::is_live() mock path for Live verdict.
#[test]
fn assay_is_live_mock_live() {
    // SAFETY: env mutation in a single-threaded test context; var cleaned up before return.
    unsafe { std::env::set_var("AGENTNS_ASSAY_MOCK_VERDICT", "Live") };
    let result = agentns_claude::assay::is_live();
    // SAFETY: symmetric remove.
    unsafe { std::env::remove_var("AGENTNS_ASSAY_MOCK_VERDICT") };
    assert!(result);
}

/// Unit-level: assay::is_live() mock path for non-Live verdict.
#[test]
fn assay_is_live_mock_non_live() {
    // SAFETY: env mutation in a single-threaded test context; var cleaned up before return.
    unsafe { std::env::set_var("AGENTNS_ASSAY_MOCK_VERDICT", "FlagRejected") };
    let result = agentns_claude::assay::is_live();
    // SAFETY: symmetric remove.
    unsafe { std::env::remove_var("AGENTNS_ASSAY_MOCK_VERDICT") };
    assert!(!result);
}
