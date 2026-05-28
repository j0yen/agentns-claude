//! AC7: [boot] prctl(PR_SET_AGENT_INTENT_TAG) reflects --intent.

#[test]
#[ignore = "boot-gated: requires linux-wintermute. iter-2 wires the real syscall."]
fn intent_tag_reflects_argv() {
    // Will assert `agentns-claude --intent /build -- cat
    // /proc/self/agent_intent_tag` prints `/build`.
}
