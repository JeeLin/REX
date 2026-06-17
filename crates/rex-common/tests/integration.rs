use rex_common::supervisor::{run_supervisor_with, SupervisorConfig};
use std::time::Duration;

#[test]
fn supervisor_with_exit_zero_stops() {
    let config = SupervisorConfig {
        restart_delay: Duration::from_millis(10),
    };
    let result = run_supervisor_with(
        || {
            let mut cmd = std::process::Command::new("sh");
            cmd.arg("-c").arg("exit 0");
            cmd
        },
        config,
    );
    assert!(result.is_ok());
}

#[test]
fn supervisor_with_exit_nonzero_restarts_then_stops() {
    let config = SupervisorConfig {
        restart_delay: Duration::from_millis(10),
    };

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let call_count = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&call_count);

    let result = run_supervisor_with(
        move || {
            let n = count.fetch_add(1, Ordering::SeqCst);
            let mut cmd = std::process::Command::new("sh");
            cmd.arg("-c");
            if n < 2 {
                cmd.arg("exit 1");
            } else {
                cmd.arg("exit 0");
            }
            cmd
        },
        config,
    );

    assert!(result.is_ok());
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}
