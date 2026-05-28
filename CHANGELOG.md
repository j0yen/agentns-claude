# Changelog

All notable changes to `agentns-claude` will be documented here. The format
is loosely based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

## [0.1.0] — 2026-05-28

Initial scaffold via `/autobuilder` Stages 1+2 from
`PRD-agentns-claude.md`. Argv surface and module skeleton; mock-mode
resolver, unshare/prctl plumbing, and budget parser land across
subsequent iterate-and-prove iterations.

### Added

- Cargo crate `agentns-claude` (Rust 2024, MSRV 1.85, dual MIT/Apache-2.0).
- `clap`-driven argv: `--intent` (required), `--budget`, `--no-unshare`,
  `--verbose`, and a trailing `-- <cmd> [args...]`.
- Module skeleton: `src/{mock,unshare,budget}.rs`.
- Acceptance test stubs (`#[ignore]`) for AC1-AC10; AC10 (README +
  CHANGELOG) is the only AC passing on iter-1.
- Read-only autobuilder artifacts: `agent/intent-card.json`,
  `rust-toolchain.toml`, `Cargo.toml` with strict BAD_RUST lints.
