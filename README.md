# agentns-claude

Wraps a command in a wintermute agent namespace so `/proc/$PID/agent_session`
reads stably from session birth. Built for Claude Code sessions; works for any
command.

## Status

**v0.1 scaffold (Stage-2 of autobuilder).** Argv surface and module skeleton
landed; the unshare/prctl plumbing and mock-mode resolver land in iter-2.
ACs 5-8 are boot-gated and require booting into `linux-wintermute`.

## Install

```
cargo install --path .
# or, from a checkout:
cargo build --release && install -Dm755 target/release/agentns-claude ~/.local/bin/agentns-claude
```

The binary depends only on glibc and a libc-style `unshare`/`prctl` surface;
on stock kernels without `CLONE_NEWAGENT`, pass `--no-unshare` to fall back
to userspace session-id synthesis.

## Usage

```
agentns-claude --intent <tag> [--budget <spec>] [--no-unshare] [--verbose] -- <cmd> [args...]
```

- `--intent <tag>` (required) — free-form string written to
  `prctl(PR_SET_AGENT_INTENT_TAG)`. Conventions: `/build`, `/dream`,
  `/self-review`, `interactive`, `headless`, `headless:<service-name>`.
- `--budget <spec>` — comma-separated `key=value` pairs:
  `wall=3600s,syscalls=1e7,write_bytes=10G,fork=1000`. Maps to
  `prctl(PR_SET_AGENT_BUDGET_LIMITS)`. SIGTERM on soft limit, SIGKILL on hard.
- `--no-unshare` — skip the unshare; emit a stderr warning and synthesize a
  session_id from `(uid, boot_time_ns, monotonic_now_ns)`. For stock kernels.
- `--verbose` — log session_id, intent_tag, and budget settings to stderr
  before exec.
- `-- <cmd> [args...]` — the wrapped command. Typically `claude`, but the
  launcher is generic.

### Examples

```
agentns-claude --intent /build -- claude
agentns-claude --intent headless:self-review --budget wall=600s -- /home/jsy/.local/bin/self-review-headless.sh
agentns-claude --intent test --no-unshare -- bash
```

## Mock mode (pre-boot iteration)

When the wintermute kernel isn't booted, downstream tools that read
`/proc/$PID/agent_session` still need a stable id to iterate against.
`agentns-claude` honors two override mechanisms:

- Env: `AGENTNS_SESSION_ID_OVERRIDE=<id>` — the launcher exports
  `AGENTNS_SESSION_ID=<id>` into the child env and skips the unshare.
- File: `/tmp/agentns-mock` containing a session-id string — the file wins
  over the env if both are set (file is more deliberate than ambient env).

Both paths emit `[agentns-claude] MOCK MODE: session_id=<id>` to stderr so
mock state is never silently mistaken for a real namespace.

## Acceptance criteria

See [`agent/intent-card.json`](agent/intent-card.json) for the structured
contract; the short version:

| AC   | Level | Test                                       | Notes                |
|------|-------|--------------------------------------------|----------------------|
| AC1  | MUST  | `tests/acceptance_build.rs`                | crate builds + ver   |
| AC2  | MUST  | `tests/acceptance_help.rs`                 | --help is honest     |
| AC3  | MUST  | `tests/acceptance_mock.rs`                 | mock-mode contract   |
| AC4  | MUST  | `tests/acceptance_exec.rs`                 | exec semantics       |
| AC5  | MUST  | `tests/acceptance_unshare_boot.rs` [boot]  | unshare lands a id   |
| AC6  | MUST  | `tests/acceptance_inheritance_boot.rs` [boot] | grandchild same id |
| AC7  | MUST  | `tests/acceptance_intent_tag_boot.rs` [boot] | intent_tag set     |
| AC8  | MUST  | `tests/acceptance_budget_boot.rs` [boot]   | budget enforced     |
| AC9  | MUST  | `tests/acceptance_no_unshare.rs`           | stock-kernel path   |
| AC10 | MUST  | `tests/acceptance_docs.rs`                 | README + CHANGELOG  |

`[boot]` tests are `#[ignore]`'d unless `WINTERMUTE_BOOT=1` is set in the
environment, so `cargo test` is green on stock kernels.

## License

Dual-licensed under MIT or Apache-2.0 at the user's option. See
[`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).
