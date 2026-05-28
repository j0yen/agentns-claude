//! AC8: [boot] budget enforcement.

#[test]
#[ignore = "boot-gated: requires linux-wintermute. iter-2 wires the real prctl."]
fn wall_budget_kills_at_limit() {
    // Will assert `agentns-claude --intent test --budget wall=2s -- sleep 60`
    // is terminated by the kernel at ~2s (SIGTERM/SIGKILL per kernel spec).
}
