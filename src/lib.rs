//! agentns-claude — wrap a command in a wintermute agent namespace.
//!
//! Stage-2 scaffold: module surface only. The iterate-and-prove loop fills in
//! mock-mode, unshare/prctl plumbing, and budget parsing across subsequent
//! iterations. See `agent/intent-card.json` for the acceptance contract.

#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod budget;
pub mod mock;
pub mod unshare;

/// Crate version surfaced through `--version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
