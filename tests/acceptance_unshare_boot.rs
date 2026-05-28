//! AC5: [boot] unshare succeeds under linux-wintermute. Gated on WINTERMUTE_BOOT=1.

#[test]
#[ignore = "boot-gated: requires linux-wintermute. iter-2 wires the real syscall."]
fn unshare_gives_fresh_session_id_per_invocation() {
    // Will assert that two consecutive invocations of
    // `agentns-claude --intent test -- cat /proc/self/agent_session`
    // produce different 128-bit hex ids.
}
