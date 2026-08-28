//! 编译期注入短 git hash（供 `version` 子命令展示）。

fn main() {
    // 优先用 CI 注入的完整 SHA 前缀
    let hash = if let Ok(sha) = std::env::var("GITHUB_SHA") {
        sha.get(..7).unwrap_or(&sha).to_string()
    } else {
        std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    };

    println!("cargo:rustc-env=REX_GIT_HASH={hash}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
}
