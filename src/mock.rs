//! Mock-mode session_id resolution.
//!
//! Stage-2 scaffold: module surface declared. The resolver (env override +
//! /tmp/agentns-mock file precedence) lands in iter-2 with its own
//! acceptance test (`tests/acceptance_mock.rs`).

/// Marker so the test surface compiles before the real resolver lands.
#[must_use]
pub const fn is_stub() -> bool {
    true
}
