//! AC4: exec semantics. Stage-2 scaffold stub; iter-2 lands the real test.

#[test]
#[ignore = "iter-1 scaffold: exec semantics land in iter-2"]
fn exit_code_is_inherited() {
    // Will assert exit 0 for `-- echo hi`, exit 1 for `-- false`, non-zero
    // with a clear error for `-- nonexistent`.
}
