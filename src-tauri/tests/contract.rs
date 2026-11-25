//! Contract tests for the real ports.
//!
//! The core is tested against substitute ports. The tests in this group check
//! that the real ports (Steam installation, process list, secret store, ...)
//! honour the same contract on a real machine. They touch the real operating
//! system, so they are not part of `cargo test` and only run on demand:
//!
//! ```text
//! cargo test -p nicelibrary --features contract-tests --test contract
//! ```
//!
//! CI runs them on the Windows runner.

#[test]
fn the_real_ports_are_tested_on_the_os_they_are_built_for() {
    assert!(
        cfg!(windows),
        "version 1 ships real ports for Windows only (ADR-0001); run contract tests on Windows"
    );
}
