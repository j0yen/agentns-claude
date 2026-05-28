//! Agent-namespace creation and stock-kernel fallback.
//!
//! Real `unshare(CLONE_NEWAGENT)` + `prctl(PR_SET_AGENT_INTENT_TAG)` plumbing
//! is wintermute-kernel-specific and lands in iter-3 once the kernel surface
//! ships in linux-wintermute. iter-2 ships:
//!
//! * a runtime detector (`kernel_has_agent_ns`) that reads
//!   `/proc/self/agent_session` — the kernel populates that file only on
//!   wintermute, so its mere existence is a reliable, syscall-free probe;
//! * `synthesize_session_id`, which the `--no-unshare` path uses to mint a
//!   128-bit id from `(uid, boot_time_s, monotonic_now_ns)` so downstream
//!   consumers still get something stable per session under stock kernels.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use nix::time::{ClockId, clock_gettime};
use nix::unistd::getuid;

/// Returns true iff the wintermute agent-namespace surface is live on this
/// kernel. The kernel exposes `/proc/$PID/agent_session` for every PID under
/// linux-wintermute; on a stock kernel the file does not exist.
#[must_use]
pub fn kernel_has_agent_ns() -> bool {
    Path::new("/proc/self/agent_session").exists()
}

/// Mint a deterministic-per-session 128-bit id under `--no-unshare`.
///
/// Layout: `[uid:u32][boot_secs:u32][monotonic_ns:u64]`, big-endian
/// concatenation, rendered as 32 lowercase hex chars. Two consecutive
/// invocations differ in `monotonic_ns`, so downstream consumers see distinct
/// session ids even under fallback.
///
/// # Errors
/// Propagates I/O errors reading `/proc/stat`, parse failures of the `btime`
/// field, and clock-read failures.
pub fn synthesize_session_id() -> io::Result<String> {
    let uid: u32 = getuid().as_raw();
    let btime = read_btime()?;
    let mono = clock_gettime(ClockId::CLOCK_MONOTONIC)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("clock_gettime: {e}")))?;
    let mono_ns = u64::try_from(mono.tv_sec()).unwrap_or(0)
        .saturating_mul(1_000_000_000)
        .saturating_add(u64::try_from(mono.tv_nsec()).unwrap_or(0));
    let mut out = String::with_capacity(32);
    let _ = write!(out, "{uid:08x}{btime:08x}{mono_ns:016x}");
    Ok(out)
}

fn read_btime() -> io::Result<u32> {
    let body = fs::read_to_string("/proc/stat")?;
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("btime ") {
            return rest
                .trim()
                .parse::<u32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("btime parse: {e}")));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "btime line missing from /proc/stat",
    ))
}
