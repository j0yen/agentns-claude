//! --budget spec parser.
//!
//! Stage-2 scaffold: module surface declared. The comma-separated key=value
//! parser (wall, syscalls, write_bytes, fork) and the --budget-file JSON
//! escape land in iter-2 alongside the prctl plumbing in
//! [`crate::unshare`].

/// Marker so the budget tests compile before the parser lands.
#[must_use]
pub const fn is_stub() -> bool {
    true
}
