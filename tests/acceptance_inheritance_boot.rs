//! AC6: [boot] inheritance: grandchildren see the same agent_session id.

#[test]
#[ignore = "boot-gated: requires linux-wintermute. iter-2 wires the real syscall."]
fn grandchild_inherits_session_id() {
    // Will assert `agentns-claude --intent test -- bash -c
    // 'cat /proc/self/agent_session; sh -c "cat /proc/self/agent_session"'`
    // prints the same id twice.
}
