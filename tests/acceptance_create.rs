//! AC1: error-mapping for create_agent_ns() against injected errnos.
//!
//! Uses `AGENTNS_PRCTL_MOCK_ERRNO` to exercise the error-mapping in
//! unshare::create_agent_ns without requiring a wintermute kernel.

#![allow(clippy::unwrap_used, clippy::panic, unsafe_code)]

use agentns_claude::unshare::{AgentNsError, create_agent_ns};

/// Helper: call create_agent_ns() with the mock errno set to `val` and unset
/// it afterwards so tests don't bleed into each other.
fn with_mock_errno<F: FnOnce(Result<(), AgentNsError>)>(val: &str, f: F) {
    // SAFETY: tests run with --test-threads=1 or are otherwise serialised by
    // the test harness on this environment; no concurrent threads read this env var.
    unsafe { std::env::set_var("AGENTNS_PRCTL_MOCK_ERRNO", val) };
    let result = create_agent_ns();
    // SAFETY: same as above.
    unsafe { std::env::remove_var("AGENTNS_PRCTL_MOCK_ERRNO") };
    f(result);
}

#[test]
fn mock_errno_zero_is_ok() {
    with_mock_errno("0", |r| {
        assert!(r.is_ok(), "errno=0 must map to Ok(()): {r:?}");
    });
}

#[test]
fn mock_errno_enosys_maps_to_kernel_lacks_prctl() {
    with_mock_errno("ENOSYS", |r| {
        match r {
            Err(AgentNsError::KernelLacksPrctl) => {}
            other => panic!("ENOSYS must map to KernelLacksPrctl, got {other:?}"),
        }
    });
}

#[test]
fn mock_errno_einval_maps_to_kernel_lacks_prctl() {
    with_mock_errno("EINVAL", |r| {
        match r {
            Err(AgentNsError::KernelLacksPrctl) => {}
            other => panic!("EINVAL must map to KernelLacksPrctl, got {other:?}"),
        }
    });
}

#[test]
fn mock_errno_eperm_maps_to_needs_cap_sys_admin() {
    with_mock_errno("EPERM", |r| {
        match r {
            Err(AgentNsError::NeedsCapSysAdmin) => {}
            other => panic!("EPERM must map to NeedsCapSysAdmin, got {other:?}"),
        }
    });
}

#[test]
fn agent_ns_error_io_round_trips() {
    let orig = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "test");
    let wrapped = AgentNsError::Io(orig);
    let io_err: std::io::Error = wrapped.into();
    assert_eq!(io_err.kind(), std::io::ErrorKind::BrokenPipe);
}

#[test]
fn agent_ns_error_kernel_lacks_prctl_converts_to_unsupported() {
    let io_err: std::io::Error = AgentNsError::KernelLacksPrctl.into();
    assert_eq!(io_err.kind(), std::io::ErrorKind::Unsupported);
}

#[test]
fn agent_ns_error_needs_cap_converts_to_permission_denied() {
    let io_err: std::io::Error = AgentNsError::NeedsCapSysAdmin.into();
    assert_eq!(io_err.kind(), std::io::ErrorKind::PermissionDenied);
}

#[test]
fn agent_ns_error_zero_converts_to_invalid_data() {
    let io_err: std::io::Error = AgentNsError::CreateSucceededButZero.into();
    assert_eq!(io_err.kind(), std::io::ErrorKind::InvalidData);
}
