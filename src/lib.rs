//! agentns-claude — wrap a command in a wintermute agent namespace.
//!
//! iter-2 lands the mock resolver, the stock-kernel detection + synthesized
//! session_id under `--no-unshare`, exec semantics, and the budget-spec
//! parser. The real `unshare(CLONE_NEWAGENT)` + prctl wiring lands in iter-3
//! once the wintermute kernel surface is callable from userspace.

#![cfg_attr(not(test), forbid(unsafe_code))]
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
