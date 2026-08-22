//! 构建 libre + baresip 静态链并生成 bindgen 绑定。
//!
//! 所有原生构建产物收敛到 `OUT_DIR`（Cargo 标准构建根，已被 gitignore），
//! 仓库内不残留任何 scratch 目录。
//!
//! 构建链（baresip v4.x 用 CMake）：
//!   1. 在 `OUT_DIR/re/build` 构建并 install libre 静态库 `libre.a`
//!      —— 必须放在 `re/build` 而非任意目录：baresip 的 `FindRE.cmake`
//!      用 `HINTS ../re/build`（相对 baresip 构建目录解析），只有 `re/build`
//!      能满足该相对路径，否则干净 CI 上 `find_package(RE REQUIRED)` 直接失败。
//!   2. baresip 配置时以 `CMAKE_PREFIX_PATH=<re/build>` 让 `FindRE.cmake` 找到
//!      libre.a，并以 `-Dre_DIR=<re/cmake>` 定位 libre 的 CMake 配置
//!      （OUT_DIR 布局下 `../re/build`、`../re/cmake` 相对路径无法落到源码目录）。
//!   3. 静态构建 `libbaresip.a`，仅链接 REX 实际用到的最小模块集
//!      （不引入 ffmpeg/opus/alsa/pulse 等外部编解码器，与上游静态构建对齐）。
//!   4. bindgen 由 `baresip/include/baresip.h` + libre 头生成 `bindings.rs`

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let re_src = manifest.join("re");
    let baresip_src = manifest.join("baresip");
    // baresip 的 FindRE.cmake 用 `HINTS ../re/build`（相对 baresip 构建目录），
    // 因此 re 必须构建在 <OUT_DIR>/re/build。
    let re_build = out_dir.join("re").join("build");
    let baresip_build = out_dir.join("baresip-build");
    // libre 的 CMake 配置文件所在目录（re-config.cmake），用于 -Dre_DIR。
    let re_cmake_dir = re_src.join("cmake");

    println!("cargo:rerun-if-changed=re");
    println!("cargo:rerun-if-changed=baresip");
    println!("cargo:rerun-if-changed=build.rs");

    let nproc = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    // 静态链接原生依赖，使最终二进制可直接运行、不依赖目标机库。
    // 影响 cmake（re/baresip 的 find_package(OpenSSL/ZLIB)）与 bindgen 的 clang。
    std::env::set_var("OPENSSL_STATIC", "1");
    std::env::set_var("ZLIB_STATIC", "1");

    // REX 实际用到的 baresip 模块（静态构建最小化集，避免拉外部编解码依赖）。
    // dtls_srtp/srtp：媒体加密；ice：NAT 穿透；
    // auconv/auresamp/aufile/ausine/mixausrc/g711/l16：音频处理框架；
    // fakevideo：占位视频源（REX 自注册 vidsrc/vidisp，不使用 baresip 音频设备）。
    let modules = "g711;l16;ausine;fakevideo;auconv;auresamp;dtls_srtp;srtp;aufile;ice;mixausrc";

    // 1) libre: configure + build static only
    if !re_build.join("libre.a").exists() {
        run(Command::new("cmake").args([
            "-S",
            re_src.to_str().unwrap(),
            "-B",
            re_build.to_str().unwrap(),
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
    }

    // 2) baresip: configure (find libre via CMAKE_PREFIX_PATH=<re_build>) + build (static)
    //    baresip 的 FindRE.cmake 用 `HINTS ../re/build`（相对 baresip 构建目录），
    //    OUT_DIR 布局下该相对路径无法落到我们的 re 构建目录，故用 CMAKE_PREFIX_PATH
    //    显式指向 re_build，使 find_library(RE_LIBRARY) 能找到 libre.a。
    //    re_DIR 额外锁定 libre 的 CMake 配置（re-config.cmake），供 find_package(re CONFIG)。
    if !baresip_build.join("libbaresip.a").exists() {
        run(Command::new("cmake").args([
            "-S",
            baresip_src.to_str().unwrap(),
            "-B",
            baresip_build.to_str().unwrap(),
            &format!("-DCMAKE_PREFIX_PATH={}", re_build.to_str().unwrap()),
            &format!("-Dre_DIR={}", re_cmake_dir.to_str().unwrap()),
            "-DSTATIC=ON",
            "-DCMAKE_BUILD_TYPE=Release",
            &format!("-DMODULES={modules}"),
        ]));
        run(Command::new("cmake").args([
            "--build",
            baresip_build.to_str().unwrap(),
            "-j",
            &nproc.to_string(),
        ]));
    }

    // 3) link static libs
    println!(
        "cargo:rustc-link-search=native={}",
        baresip_build.to_str().unwrap()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        re_build.to_str().unwrap()
    );
    println!("cargo:rustc-link-lib=static=baresip");
    println!("cargo:rustc-link-lib=static=re");
    // 系统库：libre/baresip 依赖项。
    // ssl/crypto/z 静态链接，使最终二进制自包含、可直接运行，不依赖目标机安装。
    // 用 pkg-config 探测并自动 emit 正确的 -L（原生库搜索路径）与 -l=static，
    // 避免手动 rustc-link-lib=static=ssl 在缺省搜索路径外（macOS/CI/交叉）报
    // “could not find native static library `ssl`” 的问题。
    // pthread/dl/m 属于 libc，无静态归档，保持动态（libc 本身必然存在于目标机）。
    if target_os == "windows" {
        for lib in [
            "ws2_32", "wsock32", "crypt32", "gdi32", "winmm", "iphlpapi", "dbghelp", "bcrypt",
        ] {
            println!("cargo:rustc-link-lib={lib}");
        }
        // Windows 上 OpenSSL 多为静态导入库（libssl.lib/libcrypto.lib），按静态链接。
        for lib in ["ssl", "crypto", "z"] {
            println!("cargo:rustc-link-lib=static={lib}");
        }
    } else {
        // macOS 上 Homebrew openssl 不在默认搜索路径，pkg-config 能给出正确 -L/-I。
        let _ = pkg_config::Config::new()
            .statik(true)
            .probe("openssl")
            .map_err(|e| eprintln!("pkg-config openssl: {e} (falling back to bare -l)"));
        let _ = pkg_config::Config::new()
            .statik(true)
            .probe("zlib")
            .map_err(|e| eprintln!("pkg-config zlib: {e} (falling back to bare -l)"));
        for lib in ["pthread", "dl", "m"] {
            println!("cargo:rustc-link-lib={lib}");
        }
        if target_os == "macos" {
            println!("cargo:rustc-link-lib=framework=SystemConfiguration");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
        }
    }

    // 4) bindgen（baresip.h 依赖 re.h 先 include，故用包装头）
    let wrapper = out_dir.join("rex-sip-wrapper.h");
    // 注意：`re.h` 不拉入 libre 的 `rem` 音频/视频帧模块（rem.h → rem_audio.h 等），
    // 而 `auframe`/`aufmt` 类型定义在 rem 模块中。若只 include re.h + baresip.h，
    // bindgen 会把 `auframe` 收紧为不透明零大小结构体、且看不到 `aufmt` 枚举，
    // 导致音频回调里无法读取 `auf->sampv`/`sampc`。故此处额外 include `rem.h`，
    // 使音频帧类型能被完整解析（M82b 音频驱动接管所需）。
    std::fs::write(
        &wrapper,
        "#include <re.h>\n#include <rem.h>\n#include <baresip.h>\n",
    )
    .expect("write wrapper header");
    let bindings = bindgen::Builder::default()
        .header(wrapper.to_str().unwrap())
        .clang_arg("-includestdint.h")
        .clang_arg("-includestdbool.h")
        .clang_arg("-includestring.h")
        .clang_arg("-includestddef.h")
        .clang_arg(format!(
            "-I{}",
            baresip_src.join("include").to_str().unwrap()
        ))
        .clang_arg(format!("-I{}", re_src.join("include").to_str().unwrap()))
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
        .allowlist_type("aufmt")
        .allowlist_type("ausrc")
        .allowlist_type("auplay")
        .allowlist_type("ausrc_st")
        .allowlist_type("auplay_st")
        .allowlist_type("ausrc_prm")
        .allowlist_type("auplay_prm")
        .allowlist_type("audio")
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
