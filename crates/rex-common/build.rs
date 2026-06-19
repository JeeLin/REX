use std::process::Command;

fn main() {
    // Git commit hash
    let git_commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit);

    // Build time — use git commit date for reproducibility
    let build_time = Command::new("git")
        .args(["log", "-1", "--format=%ci"])
        .output()
        .map(|o| {
            let raw = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if raw.is_empty() { "unknown".to_string() } else { raw }
        })
        .unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);

    // Re-run if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
