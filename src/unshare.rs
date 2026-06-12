//! Agent-namespace creation and stock-kernel fallback.
//!
//! Real `prctl(PR_SET_AGENT_NS)` plumbing is wintermute-kernel-specific.
//! This module (iter-3) wires the full prctl path:
//!
//! * [`create_agent_ns`] — calls `prctl(PR_SET_AGENT_NS, 0, 0, 0, 0)` via
//!   libc; maps ENOSYS/EINVAL to [`AgentNsError::KernelLacksPrctl`] and EPERM
//!   to [`AgentNsError::NeedsCapSysAdmin`];
//! * [`read_agent_session`] — reads `/proc/self/agent_session`, validates 32
//!   lowercase hex chars, rejects all-zeros;
//! * [`set_intent`] — calls `prctl(PR_SET_AGENT_INTENT_TAG, tag_ptr…)`;
//!   non-fatal on failure;
//! * [`synthesize_session_id`] — fallback for stock kernels (unchanged from
//!   iter-2);
//! * [`kernel_has_agent_ns`] — stock-kernel probe (unchanged from iter-2).
//!
//! # Kernel constants (from `uapi/linux/agent_namespaces.h`)
//!
//! ```text
//! PR_AGENT_BASE           = 0x41544E53  ("ATNS")
//! PR_SET_AGENT_NS         = PR_AGENT_BASE + 7 = 0x41544E5A
//! PR_GET_AGENT_SESSION_ID = PR_AGENT_BASE + 1 = 0x41544E54
//! PR_SET_AGENT_INTENT_TAG = PR_AGENT_BASE + 2 = 0x41544E55
//! ```

use std::borrow::Cow;
use std::ffi::CString;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;

use nix::time::{ClockId, clock_gettime};
use nix::unistd::getuid;

// ---------------------------------------------------------------------------
// Kernel constants
// ---------------------------------------------------------------------------

/// `PR_AGENT_BASE = 0x41544E53` ("ATNS"), wintermute UAPI.
const PR_AGENT_BASE: libc::c_int = 0x41544E53_u32 as libc::c_int;

/// `PR_SET_AGENT_NS = PR_AGENT_BASE + 7` — create a fresh agent namespace.
const PR_SET_AGENT_NS: libc::c_int = PR_AGENT_BASE.wrapping_add(7);

/// `PR_SET_AGENT_INTENT_TAG = PR_AGENT_BASE + 2` — write the intent tag.
const PR_SET_AGENT_INTENT_TAG: libc::c_int = PR_AGENT_BASE.wrapping_add(2);

// ---------------------------------------------------------------------------
// Typed error
// ---------------------------------------------------------------------------

/// Errors from the prctl-based agent-namespace creation path.
#[derive(Debug)]
pub enum AgentNsError {
    /// The kernel does not implement `PR_SET_AGENT_NS` (ENOSYS or EINVAL).
    KernelLacksPrctl,
    /// The caller lacks `CAP_SYS_ADMIN` in its user namespace (EPERM).
    NeedsCapSysAdmin,
    /// `prctl` returned success but `/proc/self/agent_session` is all-zeros —
    /// the namespace was not initialised properly.
    CreateSucceededButZero,
    /// Underlying I/O error.
    Io(io::Error),
}

impl std::fmt::Display for AgentNsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KernelLacksPrctl => {
                write!(f, "kernel lacks prctl(PR_SET_AGENT_NS): ENOSYS/EINVAL")
            }
            Self::NeedsCapSysAdmin => {
                write!(
                    f,
                    "prctl(PR_SET_AGENT_NS) requires CAP_SYS_ADMIN in the user namespace"
                )
            }
            Self::CreateSucceededButZero => {
                write!(
                    f,
                    "prctl(PR_SET_AGENT_NS) succeeded but agent_session is all-zeros"
                )
            }
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for AgentNsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for AgentNsError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<AgentNsError> for io::Error {
    fn from(e: AgentNsError) -> Self {
        match e {
            AgentNsError::Io(inner) => inner,
            AgentNsError::KernelLacksPrctl => {
                Self::new(io::ErrorKind::Unsupported, e.to_string())
            }
            AgentNsError::NeedsCapSysAdmin => {
                Self::new(io::ErrorKind::PermissionDenied, e.to_string())
            }
            AgentNsError::CreateSucceededButZero => {
                Self::new(io::ErrorKind::InvalidData, e.to_string())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// prctl: create_agent_ns
// ---------------------------------------------------------------------------

/// Create a fresh agent namespace on the calling task via
/// `prctl(PR_SET_AGENT_NS, 0, 0, 0, 0)`.
///
/// On success the kernel installs a new agent NS on `nsproxy`;
/// `/proc/self/agent_session` will reflect the new 128-bit session id.
///
/// # Errors
///
/// * [`AgentNsError::KernelLacksPrctl`] — kernel ENOSYS or EINVAL (no wintermute)
/// * [`AgentNsError::NeedsCapSysAdmin`] — kernel EPERM
/// * [`AgentNsError::Io`] — other OS error
///
/// # Mock injection (tests)
///
/// When `AGENTNS_PRCTL_MOCK_ERRNO` is set the function returns the
/// corresponding variant without calling the real prctl:
/// `"ENOSYS"`, `"EINVAL"` → [`AgentNsError::KernelLacksPrctl`];
/// `"EPERM"` → [`AgentNsError::NeedsCapSysAdmin`];
/// `"0"` or empty → success.
#[allow(unsafe_code)]
// SAFETY: prctl(PR_SET_AGENT_NS, 0, 0, 0, 0) has no pointer arguments.
// The kernel copies no userspace memory; it only modifies the task's nsproxy.
// The call is idempotent if a new NS cannot be nested further. All arguments
// are integer literals; no aliasing or lifetime concerns arise.
pub fn create_agent_ns() -> Result<(), AgentNsError> {
    // Test seam: AGENTNS_PRCTL_MOCK_ERRNO lets integration tests inject errno
    // values without a wintermute kernel. Present in all builds (not just
    // #[cfg(test)]) because integration tests link the library in non-test mode.
    // In production this env var is never set, so the branch is never taken.
    if let Ok(val) = std::env::var("AGENTNS_PRCTL_MOCK_ERRNO") {
        return match val.trim() {
            "0" | "" => Ok(()),
            "ENOSYS" | "EINVAL" => Err(AgentNsError::KernelLacksPrctl),
            "EPERM" => Err(AgentNsError::NeedsCapSysAdmin),
            other => Err(AgentNsError::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("AGENTNS_PRCTL_MOCK_ERRNO: unknown value {other:?}"),
            ))),
        };
    }

    // SAFETY: see item-level comment above.
    let rc = unsafe {
        libc::prctl(PR_SET_AGENT_NS, 0_usize, 0_usize, 0_usize, 0_usize)
    };

    if rc == 0 {
        return Ok(());
    }

    let errno = io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EIO);
    match errno {
        libc::ENOSYS | libc::EINVAL => Err(AgentNsError::KernelLacksPrctl),
        libc::EPERM => Err(AgentNsError::NeedsCapSysAdmin),
        _ => Err(AgentNsError::Io(io::Error::from_raw_os_error(errno))),
    }
}

// ---------------------------------------------------------------------------
// procfs read-back
// ---------------------------------------------------------------------------

/// Read the agent session id from `path` (typically `/proc/self/agent_session`).
///
/// Validates that the content is exactly 32 lowercase hex characters and that
/// the value is not all-zeros (uninitialized namespace).
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] if the content fails validation.
///
/// # Test path override
///
/// In test builds, `AGENTNS_SESSION_PATH` overrides the `path` argument so
/// unit tests can inject arbitrary content without a wintermute kernel.
pub fn read_agent_session(path: &Path) -> io::Result<String> {
    // AGENTNS_SESSION_PATH lets integration tests inject arbitrary content
    // without a wintermute kernel. Present in all builds — in production this
    // env var is never set, so the override is never taken.
    let effective: Cow<'_, Path> = if let Ok(override_str) = std::env::var("AGENTNS_SESSION_PATH") {
        Cow::Owned(std::path::PathBuf::from(override_str))
    } else {
        Cow::Borrowed(path)
    };

    let raw = fs::read_to_string(effective.as_ref())?;
    let id = raw.trim();

    if id.len() != 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "agent_session: expected 32 hex chars, got {} in {:?}",
                id.len(),
                effective
            ),
        ));
    }
    // The kernel always writes lowercase hex; uppercase is not a valid format.
    if !id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "agent_session: non-lowercase-hex character in id (expected 0-9, a-f)",
        ));
    }
    if id == "00000000000000000000000000000000" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "agent_session: all-zero id — namespace was not initialised",
        ));
    }
    Ok(id.to_owned())
}

// ---------------------------------------------------------------------------
// prctl: set_intent (non-fatal)
// ---------------------------------------------------------------------------

/// Write an intent tag into the current agent namespace via
/// `prctl(PR_SET_AGENT_INTENT_TAG, ptr, 0, 0, 0)`. Non-fatal: silently
/// ignores failure so callers continue with a valid session even on older
/// wintermute kernels that haven't landed the intent-tag extension.
///
/// The tag is NUL-terminated; the kernel silently truncates to 63 bytes.
/// Tags containing interior NUL bytes are silently skipped.
#[allow(unsafe_code)]
// SAFETY: `cstr.as_ptr()` points to a valid NUL-terminated C string that
// lives for the duration of the prctl call. The kernel reads up to 63 bytes
// and does not retain the pointer. No aliasing concerns; the string is
// stack-local.
pub fn set_intent(tag: &str) {
    let Ok(cstr) = CString::new(tag) else {
        // Interior NUL — cannot represent in a C string; skip silently.
        return;
    };

    // SAFETY: see item-level comment above.
    let _rc = unsafe {
        libc::prctl(
            PR_SET_AGENT_INTENT_TAG,
            cstr.as_ptr() as usize,
            0_usize,
            0_usize,
            0_usize,
        )
    };
    // Non-fatal: ignore return code.
}

// ---------------------------------------------------------------------------
// Detection / fallback (iter-2, unchanged)
// ---------------------------------------------------------------------------

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
