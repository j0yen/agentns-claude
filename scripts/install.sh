#!/usr/bin/env bash
# scripts/install.sh — idempotent install of agentns-claude with cap_sys_admin+ep.
#
# Usage:
#   bash scripts/install.sh            # build + install + setcap
#   bash scripts/install.sh --dry-run  # print what would happen, do nothing
#
# Prerequisites:
#   - Rust toolchain (cargo in PATH or rustup)
#   - sudo + libcap (setcap/getcap)
#   - Run from the agentns-claude repo root
set -euo pipefail

DEST="$HOME/.local/bin/agentns-claude"
DRY_RUN=false
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=true ;;
    *) echo "usage: install.sh [--dry-run]" >&2; exit 1 ;;
  esac
done

log()  { echo "[install] $*"; }
logd() { echo "[install] (dry-run) $*"; }

# ---- version probe ----
# Compare the version embedded in the installed binary against the crate version.
# Skip rebuild + reinstall if they match and the binary is already capped.
CRATE_VERSION="$(grep -m1 '^version = ' "$REPO_ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
installed_version=""
if [[ -x "$DEST" ]]; then
  installed_version="$("$DEST" --version 2>/dev/null | awk '{print $NF}' || true)"
fi
installed_cap=""
if [[ -x "$DEST" ]]; then
  installed_cap="$(getcap "$DEST" 2>/dev/null || true)"
fi

if [[ "$installed_version" == "$CRATE_VERSION" ]] && \
   [[ "$installed_cap" == *"cap_sys_admin=ep"* ]]; then
  log "already current (v$CRATE_VERSION, cap_sys_admin=ep) — nothing to do"
  exit 0
fi

# ---- build ----
log "building agentns-claude v$CRATE_VERSION (release)…"
if [[ "$DRY_RUN" == true ]]; then
  logd "cargo build --release  (in $REPO_ROOT)"
else
  (cd "$REPO_ROOT" && cargo build --release 2>&1)
fi

BUILT_BIN="$REPO_ROOT/target/release/agentns-claude"
if [[ "$DRY_RUN" == false ]] && [[ ! -f "$BUILT_BIN" ]]; then
  echo "[install] error: build succeeded but $BUILT_BIN not found" >&2
  exit 1
fi

# ---- install ----
log "installing to $DEST…"
if [[ "$DRY_RUN" == true ]]; then
  logd "install -Dm755 $BUILT_BIN $DEST"
else
  install -Dm755 "$BUILT_BIN" "$DEST"
fi

# ---- setcap ----
# File capabilities are stripped on every install/copy, so this must re-run each time.
log "applying cap_sys_admin+ep…"
if [[ "$DRY_RUN" == true ]]; then
  logd "sudo setcap cap_sys_admin+ep $DEST"
else
  sudo setcap cap_sys_admin+ep "$DEST"
fi

# ---- verify ----
log "verifying…"
if [[ "$DRY_RUN" == true ]]; then
  logd "getcap $DEST  →  (would verify cap_sys_admin=ep)"
else
  CAPS="$(getcap "$DEST")"
  echo "[install] getcap: $CAPS"
  if [[ "$CAPS" != *"cap_sys_admin=ep"* ]]; then
    echo "[install] error: setcap did not stick — got: $CAPS" >&2
    exit 1
  fi
  VERSION_OUT="$("$DEST" --version 2>/dev/null || true)"
  echo "[install] version: $VERSION_OUT"
  log "OK — agentns-claude v$CRATE_VERSION installed with cap_sys_admin=ep"
fi
