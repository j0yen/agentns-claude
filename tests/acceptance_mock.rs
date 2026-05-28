//! AC3: mock mode. Stage-2 scaffold stub; iter-2 lands the real exec test.

#[test]
#[ignore = "iter-1 scaffold: mock-mode exec lands in iter-2"]
fn env_override_propagates_to_child() {
    // Will spawn `agentns-claude --intent test -- printenv AGENTNS_SESSION_ID`
    // under AGENTNS_SESSION_ID_OVERRIDE=deadbeef-… and assert child stdout
    // matches.
}

#[test]
#[ignore = "iter-1 scaffold: mock-mode exec lands in iter-2"]
fn file_wins_over_env() {
    // Will write /tmp/agentns-mock with a distinct id, set
    // AGENTNS_SESSION_ID_OVERRIDE with a different id, and assert the file
    // value wins.
}
