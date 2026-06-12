//! agentns-claude — wrap a command in a wintermute agent namespace.
//!
//! iter-3 wires the real `prctl(PR_SET_AGENT_NS)` creation path via libc,
//! with ENOSYS/EINVAL fallback to synthesized ids on stock kernels.
//! iter-2 shipped the mock resolver, `--no-unshare` fallback, exec semantics,
//! and the budget-spec parser.

// unsafe_code is "deny" in Cargo.toml [lints.rust]. Individual items that
// need unsafe (the prctl calls in unshare.rs) carry #[allow(unsafe_code)]
// with a documented safety justification comment. The global forbid from
// iter-2 is removed to allow those local overrides.
#![allow(
    clippy::doc_markdown,
    clippy::too_long_first_doc_paragraph,
    clippy::missing_errors_doc,
    clippy::option_if_let_else
)]

pub mod budget;
pub mod mock;
pub mod unshare;

/// Crate version surfaced through `--version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
