//! AC2: read_agent_session() validation.
//!
//! Tests that the function correctly accepts valid 32-char non-zero hex ids
//! and rejects all-zeros, wrong-length, and non-hex strings.
//! Uses `AGENTNS_SESSION_PATH` to inject a tmp file without needing a kernel.

#![allow(clippy::unwrap_used, clippy::panic, unsafe_code, clippy::expect_used)]

use std::io::Write;
use std::path::Path;

use agentns_claude::unshare::read_agent_session;

/// Write `content` to a uniquely named tmp file; return its path.
fn write_tmp(tag: &str, content: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "agentns-readback-{}-{}.tmp",
        std::process::id(),
        tag
    ));
    let mut f = std::fs::File::create(&path).expect("create tmp");
    f.write_all(content.as_bytes()).expect("write tmp");
    path
}

fn with_path_override<F: FnOnce(std::io::Result<String>)>(content: &str, tag: &str, f: F) {
    let path = write_tmp(tag, content);
    // SAFETY: tests run sequentially; no concurrent env-var readers.
    unsafe { std::env::set_var("AGENTNS_SESSION_PATH", &path) };
    let result = read_agent_session(Path::new("/proc/self/agent_session"));
    // SAFETY: same as above.
    unsafe { std::env::remove_var("AGENTNS_SESSION_PATH") };
    let _ = std::fs::remove_file(&path);
    f(result);
}

#[test]
fn accepts_valid_32_hex_nonzero() {
    with_path_override("deadbeef0123456789abcdef01234567\n", "valid", |r| {
        assert_eq!(
            r.unwrap(),
            "deadbeef0123456789abcdef01234567",
            "valid id must be returned unchanged"
        );
    });
}

#[test]
fn accepts_valid_id_without_newline() {
    with_path_override("deadbeef0123456789abcdef01234567", "nonl", |r| {
        assert_eq!(
            r.unwrap(),
            "deadbeef0123456789abcdef01234567"
        );
    });
}

#[test]
fn rejects_all_zeros() {
    with_path_override("00000000000000000000000000000000\n", "zeros", |r| {
        assert!(r.is_err(), "all-zero id must be rejected");
        let msg = r.unwrap_err().to_string();
        assert!(
            msg.contains("all-zero"),
            "error must mention all-zero: {msg}"
        );
    });
}

#[test]
fn rejects_too_short() {
    with_path_override("deadbeef\n", "short", |r| {
        assert!(r.is_err(), "short id must be rejected");
        let msg = r.unwrap_err().to_string();
        assert!(
            msg.contains("expected 32 hex"),
            "error must mention length: {msg}"
        );
    });
}

#[test]
fn rejects_too_long() {
    with_path_override("deadbeef0123456789abcdef0123456700\n", "long", |r| {
        assert!(r.is_err(), "long id must be rejected");
    });
}

#[test]
fn rejects_non_hex_chars() {
    with_path_override("deadbeef0123456789abcdefghijklmn\n", "nonhex", |r| {
        assert!(r.is_err(), "non-hex id must be rejected");
        let msg = r.unwrap_err().to_string();
        assert!(
            msg.contains("non-hex") || msg.contains("hex"),
            "error must mention hex: {msg}"
        );
    });
}

#[test]
fn rejects_uppercase_hex() {
    // The kernel writes lowercase; uppercase is not a valid id format.
    with_path_override("DEADBEEF0123456789ABCDEF01234567\n", "upper", |r| {
        assert!(r.is_err(), "uppercase hex must be rejected (kernel writes lowercase)");
    });
}
