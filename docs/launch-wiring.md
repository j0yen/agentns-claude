# agentns-claude Launch Wiring

Every place agentns-claude intercepts a Claude session, how to verify, and
what the fallback path is when the binary isn't installed or the kernel doesn't
support agent namespaces.

---

## Install sites

### 1. Interactive shell — `~/.zshrc` `claude()` function

**File:** `~/.zshrc` (lines ~31–37)

```zsh
claude() {
  if [[ -x "$HOME/.local/bin/agentns-claude" ]] && [[ -z "${AGENTNS_WRAPPED:-}" ]]; then
    AGENTNS_WRAPPED=1 "$HOME/.local/bin/agentns-claude" --intent interactive -- /home/jsy/.local/bin/claude "$@"
  else
    command claude "$@"
  fi
}
```

- **Guard:** `-x "$HOME/.local/bin/agentns-claude"` — if binary absent, falls
  through to `command claude` with no wrapper, no error.
- **Recursion guard:** `AGENTNS_WRAPPED=1` prevents re-entry if agentns-claude
  somehow exec()s back through the shell function.
- **`--no-unshare` removed** by this PRD (agentns-launch-flip). The graceful
  fallback path (synthesize + warn on stock kernels) is wired by
  `PRD-agentns-claude-prctl-wire`; without that branch landed, removing
  `--no-unshare` would cause `StockKernelNoFallback` on pre-boot kernels.
- **Post-reboot behaviour:** once the box boots into a linux-wintermute kernel
  that supports `prctl(PR_SET_AGENT_NS)`, the same wrapper starts producing
  real session IDs with no further edit.

### 2. Headless `/build` — `~/.local/bin/claude-build-tick.sh`

**Invoked by:** `claude-build-headless.sh` → detached `claude-build-work.service`

```bash
if [[ -x "$HOME/.local/bin/agentns-claude" ]]; then
  "$HOME/.local/bin/agentns-claude" --intent /build --no-unshare -- \
    /home/jsy/.local/bin/claude -p "/build"
else
  /home/jsy/.local/bin/claude -p "/build"
fi
```

**Status:** already wired, uses `--no-unshare` (fallback for stock kernel).
**Pending:** remove `--no-unshare` after prctl-wire lands (same flip as zshrc).

### 3. Headless `/dream` — `~/.local/bin/claude-dream-headless.sh`

**Invoked by:** `claude-dream.service` → `claude-dream-headless.sh`

```bash
if [[ -x "$HOME/.local/bin/agentns-claude" ]]; then
  "$HOME/.local/bin/agentns-claude" --intent /dream --no-unshare -- \
    /home/jsy/.local/bin/claude -p "/dream"
else
  /home/jsy/.local/bin/claude -p "/dream"
fi
```

**Status:** already wired, uses `--no-unshare` (fallback for stock kernel).
**Pending:** remove `--no-unshare` after prctl-wire lands.

### 4. Systemd service units (`claude-build.service`, `claude-dream.service`)

These units call the headless wrapper scripts (which already route through
agentns-claude), so they do not need direct changes to their `ExecStart`.
The wiring chain is:

```
timer → claude-build.service(ExecStart=claude-build-headless.sh)
           → systemd-run claude-build-tick.sh
               → agentns-claude --intent /build -- claude -p /build
```

```
timer → claude-dream.service(ExecStart=claude-dream-headless.sh)
           → agentns-claude --intent /dream -- claude -p /dream
```

No drop-in edits to the units are required for this PRD.

---

## setcap requirement

File capabilities are stripped on every `install`/copy. `scripts/install.sh`
re-applies `cap_sys_admin+ep` after every build + install. If the binary is
replaced without re-running `install.sh`, the file capability is lost and
agentns-claude degrades (fails or synthesizes depending on fallback path).

Verify at any time:

```bash
getcap ~/.local/bin/agentns-claude
# expected: /home/jsy/.local/bin/agentns-claude cap_sys_admin=ep
```

---

## Fallback behaviour matrix

| Condition | `kernel_has_agent_ns` | `--no-unshare` | Outcome |
|---|---|---|---|
| prctl-wire kernel | true | either | synthesizes + warning (iter-2); real ns in iter-3 |
| stock kernel | false | present | synthesize + warn (no-unshare fallback) |
| stock kernel | false | absent | `StockKernelNoFallback` error (pre-prctl-wire) |
| stock kernel | false | absent + graceful fallback wired | synthesize + warn |
| binary absent | — | — | `command claude` direct (zshrc guard) |

After prctl-wire lands, all stock-kernel + no-`--no-unshare` cases degrade
gracefully (synthesize + warn) rather than erroring.

---

## Audit checklist

Run after any zshrc/unit edit to verify coverage:

```bash
# 1. Interactive wrapper in place and --no-unshare absent
grep -n "agentns-claude\|--no-unshare" ~/.zshrc

# 2. Binary installed + capped
ls -la ~/.local/bin/agentns-claude
getcap ~/.local/bin/agentns-claude

# 3. Build tick wired
grep -n "agentns-claude\|--no-unshare" ~/.local/bin/claude-build-tick.sh

# 4. Dream headless wired
grep -n "agentns-claude\|--no-unshare" ~/.local/bin/claude-dream-headless.sh

# 5. Units route through the wrapper scripts (not direct claude)
systemctl --user cat claude-build.service claude-dream.service
```
