//! 构建 baresip + libre 静态链并生成 bindgen 绑定。
//!
//! 所有原生构建产物收敛到 `OUT_DIR`（Cargo 标准构建根，已被 gitignore），
//! 仓库内不残留任何 scratch 目录。
//!
//! 构建链（baresip v4.x 用 CMake）：
//!   1. 构建并 install libre（baresip 的依赖库，独立仓库 `baresip/re`）到 `<OUT_DIR>/re-install`
//!   2. 以 `CMAKE_PREFIX_PATH=<OUT_DIR>/re-install` 配置 baresip，使其 `find_package(re ...)`
//!      能找到已 install 的 libre（HINTS 相对路径在 prefix 模式下由 CMAKE_PREFIX_PATH 满足）
//!   3. 静态构建 libbaresip.a
//!   4. bindgen 由 `baresip/include/baresip.h` + libre 头生成 `bindings.rs`

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let re_src = manifest.join("re");
    let baresip_src = manifest.join("baresip");
    let re_install = out_dir.join("re-install");
    let re_build = out_dir.join("re-build");
    let baresip_build = out_dir.join("baresip-build");

    println!("cargo:rerun-if-changed=re");
    println!("cargo:rerun-if-changed=baresip");
    println!("cargo:rerun-if-changed=build.rs");

    let nproc = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    // 1) libre: configure + build + install (static only)
    if !re_install.join("lib/libre.a").exists() {
        run(Command::new("cmake")
            .args([
                "-S",
                re_src.to_str().unwrap(),
                "-B",
                re_build.to_str().unwrap(),
                &format!("-DCMAKE_INSTALL_PREFIX={}", re_install.to_str().unwrap()),
                "-DLIBRE_BUILD_SHARED=OFF",
                "-DLIBRE_BUILD_STATIC=ON",
                "-DCMAKE_BUILD_TYPE=Release",
            ]));
        run(Command::new("cmake").args([
            "--build",
            re_build.to_str().unwrap(),
            "-j",
            &nproc.to_string(),
        ]));
        run(Command::new("cmake").args([
            "--install",
            re_build.to_str().unwrap(),
        ]));
    }

    // 2) baresip: configure (find libre via CMAKE_PREFIX_PATH) + build (static)
    if !baresip_build.join("libbaresip.a").exists() {
        run(Command::new("cmake")
            .args([
                "-S",
                baresip_src.to_str().unwrap(),
                "-B",
                baresip_build.to_str().unwrap(),
                &format!("-DCMAKE_PREFIX_PATH={}", re_install.to_str().unwrap()),
                "-DSTATIC=ON",
                "-DCMAKE_BUILD_TYPE=Release",
            ]));
        run(Command::new("cmake").args([
            "--build",
            baresip_build.to_str().unwrap(),
            "-j",
            &nproc.to_string(),
        ]));
    }

    // link static libs (search both build dirs + install lib dir)
    println!(
        "cargo:rustc-link-search=native={}",
        baresip_build.to_str().unwrap()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        re_install.join("lib").to_str().unwrap()
    );
    println!("cargo:rustc-link-lib=static=baresip");
    println!("cargo:rustc-link-lib=re");
    // system libs libre/baresip depend on
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=m");

    // 3) bindgen（baresip.h 依赖 re.h 先 include，故用包装头）
    let wrapper = out_dir.join("rex-sip-wrapper.h");
    std::fs::write(
        &wrapper,
        "#include <re.h>\n#include <baresip.h>\n",
    )
    .expect("write wrapper header");
    let bindings = bindgen::Builder::default()
        .header(wrapper.to_str().unwrap())
        .clang_arg("-includestdint.h")
        .clang_arg("-includestdbool.h")
        .clang_arg("-includestring.h")
        .clang_arg("-includestddef.h")
        .clang_arg(format!("-I{}", baresip_src.join("include").to_str().unwrap()))
        .clang_arg(format!("-I{}", re_src.join("include").to_str().unwrap()))
        .clang_arg(format!("-I{}", re_install.join("include").to_str().unwrap()))
        .allowlist_type("ua")
        .allowlist_type("call")
        .allowlist_type("account")
        .allowlist_type("bevent")
        .allowlist_type("bevent_ev")
        .allowlist_type("call_event")
        .allowlist_type("call_state")
        .allowlist_type("vidmode")
        .allowlist_type("answermode")
        .allowlist_type("dtmfmode")
        .allowlist_type("sip_msg")
        .allowlist_type("config")
        .allowlist_type("pl")
        .allowlist_type("odict")
        .allowlist_type("re_printf")
        .allowlist_type("list")
        .allowlist_type("mbuf")
        .allowlist_type("sa")
        .allowlist_type("sip_addr")
        .allowlist_type("uri")
        .allowlist_type("stream")
        .allowlist_type("sdp_session")
        .allowlist_type("sdp_media")
        .allowlist_type("auframe")
        .allowlist_type("vidframe")
        .allowlist_type("vidrect")
        .allowlist_type("vidsz")
        .allowlist_function(".*")
        .generate_comments(false)
        .generate()
        .expect("Unable to generate baresip bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings.rs");
}

fn run(cmd: &mut std::process::Command) {
    let status = cmd.status().expect("failed to spawn build command");
    assert!(status.success(), "build command failed: {:?}", cmd);
}
