//! CLONE_NEWAGENT + prctl plumbing.
//!
//! Stage-2 scaffold: module surface declared. The syscall implementations
//! (gated on `cfg(target_os = "linux")` and the wintermute kernel) land in
//! iter-2. AC5-AC8 are `#[ignore]`'d until then because they need the live
//! kernel.

/// Marker so the boot-gated tests can compile before the real syscalls land.
#[must_use]
pub const fn is_stub() -> bool {
    true
}
