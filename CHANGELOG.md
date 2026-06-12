# Changelog

## v0.4.0 — 2026-06-12

### iter-4: assay agentns gate (agentns-session-wire AC5)

- Add `src/assay.rs` module: `is_live()` runs `assay agentns --json`, parses
  `verdict.type`, and returns `true` only when the verdict is `"Live"`.
  Mock seam: `AGENTNS_ASSAY_MOCK_VERDICT` injects verdicts in tests.
- Wire assay gate into `resolve_session()`: when `kernel_has_agent_ns()` is
  true but `assay::is_live()` is false, skip prctl and go directly to
  synthesize path with mode `"synth-fallback"` and WARN log.
- Add `tests/acceptance_assay_gate.rs`: AC5 acceptance tests covering
  FlagRejected/Inert → synth-fallback, and Live + prctl mock 0 → prctl mode.
- Update resolution order comment to iter-4 (6-step ladder with assay gate
  at step 3).

## v0.3.0 — 2026-06-12

### iter-3: prctl(PR_SET_AGENT_NS) wiring

- Add `create_agent_ns()` using `libc::prctl(PR_SET_AGENT_NS)` with typed `AgentNsError` (KernelLacksPrctl/NeedsCapSysAdmin/CreateSucceededButZero/Io)
- Add `read_agent_session()` validating 32 lowercase hex non-zero chars from procfs
- Add `set_intent()` non-fatal prctl(PR_SET_AGENT_INTENT_TAG) call
- Rewire `resolve_session()`: mock → --no-unshare → prctl (mode "prctl") → EINVAL fallback (mode "synth-fallback") → EPERM fatal
- Export `AGENTNS_MODE` to child env alongside SESSION_ID and INTENT
- Add `libc` dependency
- New tests: AC1 (create error mapping), AC2 (read_agent_session validation), AC3 (precedence), AC6 (AGENTNS_MODE in child env)
- Remove pending-unshare mode string

## v0.2.0 — 2026-06-12

install script + launch wiring: scripts/install.sh (idempotent build+install+setcap), docs/launch-wiring.md (audit of all 4 install sites). Binary installed to ~/.local/bin/agentns-claude with cap_sys_admin=ep. zshrc --no-unshare removed.

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
