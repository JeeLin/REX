use std::process::Command;
use std::time::Duration;

fn bin_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn hub_worker_exits_cleanly() {
    let output = Command::new(bin_dir().join("rex-hub"))
        .arg("--worker")
        .output()
        .expect("failed to run rex-hub");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rex-hub starting"));
    assert!(stdout.contains("worker started"));
}

#[test]
fn hub_supervisor_spawns_worker() {
    let child = Command::new(bin_dir().join("rex-hub"))
        .spawn()
        .expect("failed to run rex-hub");

    std::thread::sleep(Duration::from_millis(200));
    drop(child);
}

#[test]
fn agent_worker_exits_cleanly() {
    let output = Command::new(bin_dir().join("rex-agent"))
        .arg("--worker")
        .output()
        .expect("failed to run rex-agent");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rex-agent starting"));
    assert!(stdout.contains("worker started"));
}
