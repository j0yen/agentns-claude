//! Mock-mode session_id resolution.
//!
//! Two pre-boot escape hatches let downstream tools iterate without the
//! wintermute kernel:
//!
//! * `/tmp/agentns-mock` (overridable via `$AGENTNS_MOCK_FILE` for tests) —
//!   if the file exists and is non-empty, its trimmed contents are the
//!   session_id. File wins because it's deliberate (someone wrote it) over
//!   ambient env.
//! * `$AGENTNS_SESSION_ID_OVERRIDE` — fallback. Whatever the env var
//!   contains becomes the session_id.
//!
//! Both paths emit a `[agentns-claude] MOCK MODE: session_id=…` line to
//! stderr so a real session is never silently mistaken for mock.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Result of mock-mode resolution: the session_id plus which source supplied it.
pub struct Mock {
    /// 128-bit-ish identifier the child will see as `$AGENTNS_SESSION_ID`.
    pub session_id: String,
    /// Origin of the id: `"file"` (mock file) or `"env"` (override env var).
    pub source: &'static str,
}

fn mock_file_path() -> PathBuf {
    env::var_os("AGENTNS_MOCK_FILE")
        .map_or_else(|| PathBuf::from("/tmp/agentns-mock"), PathBuf::from)
}

/// Try to resolve a session_id from mock sources. Returns `Ok(None)` when no
/// mock source is configured (the real unshare path takes over).
///
/// # Errors
/// Propagates I/O errors reading the mock file.
pub fn resolve() -> io::Result<Option<Mock>> {
    let path = mock_file_path();
    if path.is_file() {
        let body = fs::read_to_string(&path)?;
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            return Ok(Some(Mock {
                session_id: trimmed.to_owned(),
                source: "file",
            }));
        }
    }
    if let Some(val) = env::var_os("AGENTNS_SESSION_ID_OVERRIDE") {
        let s = val.to_string_lossy().into_owned();
        if !s.is_empty() {
            return Ok(Some(Mock {
                session_id: s,
                source: "env",
            }));
        }
    }
    Ok(None)
}

/// Emit the mandatory mock-mode warning to stderr.
///
/// # Errors
/// Propagates I/O errors writing to stderr.
pub fn announce(mock: &Mock) -> io::Result<()> {
    let mut err = io::stderr().lock();
    writeln!(
        err,
        "[agentns-claude] MOCK MODE: session_id={} source={}",
        mock.session_id, mock.source
    )
}
