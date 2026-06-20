# agentns-claude

Launches a command inside a wintermute agent namespace so that `/proc/$PID/agent_session` reads a stable id from the first instruction the process runs. Built for Claude Code sessions; the launcher itself is generic.

A downstream tool that attributes work to an agent session — provenance stamping, budget accounting, reaping — needs the session id to exist *before* the wrapped process does anything. If the id appears partway through startup, the early work is unattributable. `agentns-claude` creates the namespace, sets the intent tag, and only then `exec`s the child, so the id is born with the process rather than after it.

## Install

```sh
cargo install --path .
# or, from a checkout:
cargo build --release && install -Dm755 target/release/agentns-claude ~/.local/bin/agentns-claude
```

On a wintermute kernel, creating a real namespace needs `CAP_SYS_ADMIN`; `scripts/install.sh` builds, installs, and applies the capability with `setcap`. On stock kernels there is no `PR_SET_AGENT_NS`, so the launcher synthesizes a session id instead (see below) — no capability required.

## Usage

```sh
agentns-claude --intent <tag> [--budget <spec>] [--no-unshare] [--verbose] -- <cmd> [args...]
```

- `--intent <tag>` (required) — written to `prctl(PR_SET_AGENT_INTENT_TAG)`. Conventions: `/build`, `/dream`, `/self-review`, `interactive`, `headless`, `headless:<service>`.
- `--budget <spec>` — comma-separated `key=value` pairs: `wall=3600s,syscalls=1e7,write_bytes=10G,fork=1000`. Parsed and validated now; applying it through `prctl(PR_SET_AGENT_BUDGET_LIMITS)` is pending the kernel-side prctl. A bad spec fails fast before exec; a typo'd key is rejected rather than silently dropped.
- `--no-unshare` — skip namespace creation and synthesize a session id from `(uid, boot_time_ns, monotonic_now_ns)`. For stock kernels.
- `--verbose` — log the resolved mode, session id, intent, and budget to stderr before exec.
- `-- <cmd> [args...]` — the wrapped command, typically `claude`.

```sh
agentns-claude --intent /build -- claude
agentns-claude --intent headless:self-review --budget wall=600s -- ~/.local/bin/self-review-headless.sh
agentns-claude --intent test --no-unshare -- bash
```

## How a session id is resolved

The launcher walks a fixed precedence ladder and exports the result to the child as `AGENTNS_SESSION_ID`, `AGENTNS_INTENT`, and `AGENTNS_MODE` so a tool reading the environment always knows *how* the id was obtained:

1. **Mock** — `/tmp/agentns-mock` (file wins) or `AGENTNS_SESSION_ID_OVERRIDE` (env). Mode `mock`.
2. **`--no-unshare`** — synthesize from `(uid, btime, monotonic_ns)`. Mode `no-unshare`.
3. **Assay gate** — run `assay agentns`; if it reports anything other than `Live`, skip the prctl attempt and synthesize. Mode `synth-fallback`. This is what catches the booted-but-broken kernel rather than failing in the syscall.
4. **prctl** — `create_agent_ns()` + `set_intent()` + read back a validated 32-hex non-zero id from procfs. Mode `prctl`.
5. **prctl `ENOSYS`/`EINVAL`** (stock kernel) — synthesize and warn. Mode `synth-fallback`.
6. **prctl `EPERM`** — fatal; the message tells you to re-run with `CAP_SYS_ADMIN` or `--no-unshare`.

The mode string is the honesty mechanism: a synthesized id and a real namespaced id are never indistinguishable downstream.

## Mock mode (iterating before the kernel boots)

Tools that read `/proc/$PID/agent_session` need a stable id to develop against before `linux-wintermute` is running. Set `AGENTNS_SESSION_ID_OVERRIDE=<id>`, or write a session id into `/tmp/agentns-mock` (the file wins over the env — a file is more deliberate than an ambient variable). Either path prints `[agentns-claude] MOCK MODE: session_id=<id>` to stderr, so mock state is never mistaken for a real namespace.

## Status

The launcher is complete through iter-4 (v0.4.0). The argv surface, mock resolver, `--no-unshare` synthesis, the real `prctl(PR_SET_AGENT_NS)` path, and the `assay agentns` liveness gate are all wired and tested. Two things remain kernel-gated: `--budget` is parsed and validated but not yet applied through `prctl(PR_SET_AGENT_BUDGET_LIMITS)`, and the four `[boot]` acceptance tests (`AC5`–`AC8`) are `#[ignore]`'d unless `WINTERMUTE_BOOT=1`, so `cargo test` stays green on a stock kernel.

## Where it fits

This is the userspace launcher for [`agentns`](https://github.com/j0yen/agentns), the kernel patch series that adds the agent namespace. `agentns` provides the namespace and the prctl ops; `agentns-claude` is the front door that wraps a real command in one.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
