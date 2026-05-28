//! AC9: --no-unshare fallback. Stage-2 scaffold stub; iter-2 lands the real test.

#[test]
#[ignore = "iter-1 scaffold: --no-unshare fallback lands in iter-2"]
fn no_unshare_exits_zero_with_warning() {
    // Will assert `agentns-claude --intent test --no-unshare -- true` exits
    // 0 and stderr contains a warning string ('mock' or 'no-unshare' or
    // 'stock kernel').
}

#[test]
#[ignore = "iter-1 scaffold: --no-unshare fallback lands in iter-2"]
fn missing_no_unshare_exits_nonzero_on_stock_kernel() {
    // Will assert that without --no-unshare and without the wintermute
    // kernel, the launcher exits non-zero with a clear 'kernel does not
    // support agent namespaces' message on stderr.
}
