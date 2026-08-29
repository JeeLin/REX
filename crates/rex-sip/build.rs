//! 构建 libre + baresip 静态链并生成 bindgen 绑定。
//!
//! 所有原生构建产物收敛到 `OUT_DIR`（Cargo 标准构建根，已被 gitignore），
//! 仓库内不残留任何 scratch 目录。
//!
//! 构建链（baresip v4.x 用 CMake）：
//!   1. 在 `OUT_DIR/re/build` 构建 libre 静态库
//!      —— 必须放在 `re/build` 而非任意目录：baresip 的 `FindRE.cmake`
//!      用 `HINTS ../re/build`（相对 baresip 构建目录解析），只有 `re/build`
//!      能满足该相对路径，否则干净 CI 上 `find_package(RE REQUIRED)` 直接失败。
//!   2. baresip 配置时以 `CMAKE_PREFIX_PATH=<re/build>` 让 `FindRE.cmake` 找到
//!      libre.a，并以 `-Dre_DIR=<re/cmake>` 定位 libre 的 CMake 配置。
//!   3. 静态构建 `libbaresip.a`，仅链接 REX 实际用到的最小模块集
//!      （不引入 ffmpeg/opus/alsa/pulse 等外部编解码器，与上游静态构建对齐）。
//!   4. bindgen 由 `baresip/include/baresip.h` + libre 头生成 `bindings.rs`
//!
//! 各平台均按「原生构建」组织 CI（与 baresip 官方 CI 一致）：每个目标架构用
//! 对应原生 runner（linux-arm64 → ubuntu-24.04-arm，mac-amd64 → macos-13），
//! 避免交叉编译在 zlib/openssl 架构匹配与 pkg-config 架构探测上的坑，使最终
//! 二进制可直接运行、不依赖目标机安装。

use std::path::{Path, PathBuf};
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
    // Windows 上 OpenSSL 由 choco 提供，按动态链接（见下方 link 段），故此处
    // 仅对非 Windows 设静态标志；Windows 的 re/baresip cmake 仍走 find_package。
    if target_os != "windows" {
        std::env::set_var("OPENSSL_STATIC", "1");
        std::env::set_var("ZLIB_STATIC", "1");
    }

    // REX 实际用到的 baresip 模块（静态构建最小化集，避免拉外部编解码依赖）。
    // dtls_srtp/srtp：媒体加密；ice：NAT 穿透；
    // auconv/auresamp/aufile/ausine/mixausrc/g711/l16：音频处理框架；
    // fakevideo：占位视频源（REX 自注册 vidsrc/vidisp，不使用 baresip 音频设备）。
    let modules = "g711;l16;ausine;fakevideo;auconv;auresamp;dtls_srtp;srtp;aufile;ice;mixausrc";

    // 1) libre: configure + build static only
    if find_archive(&re_build, &["libre.a", "re-static.lib"]).is_none() {
        let mut cmd = Command::new("cmake");
        cmd.args([
            "-S",
            re_src.to_str().unwrap(),
            "-B",
            re_build.to_str().unwrap(),
            "-DLIBRE_BUILD_SHARED=OFF",
            "-DLIBRE_BUILD_STATIC=ON",
            "-DCMAKE_BUILD_TYPE=Release",
        ]);
        for a in cmake_cross_args() {
            cmd.arg(a);
        }
        // Windows 上 OpenSSL 静态链入 exe：让 cmake 的 find_package(OpenSSL) 也选
        // 静态归档（libssl_static.lib/libcrypto_static.lib），否则 re/baresip 会按
        // 动态导入库编译出 __imp_ 引用，最终 exe 仍会依赖 libssl/libcrypto DLL。
        if target_os == "windows" {
            cmd.arg("-DOPENSSL_USE_STATIC_LIBS=ON");
        }
        run(&mut cmd);
        run(Command::new("cmake").args([
            "--build",
            re_build.to_str().unwrap(),
            "-j",
            &nproc.to_string(),
        ]));
    }

    // 2) baresip: configure (find libre via CMAKE_PREFIX_PATH=<re_build>) + build (static)
    if find_archive(&baresip_build, &["libbaresip.a", "libbaresip.lib"]).is_none() {
        // MSVC 多配置生成器把 re 的静态库放在 <CONFIG> 子目录（如 re/build/Debug/
        // re-static.lib），而 baresip 的 FindRE.cmake 用 `HINTS ../re/build` 只查
        // re/build 一级，找不到 <CONFIG> 下的归档。故直接用 -DRE_LIBRARY 给出
        // 实际绝对路径、-DRE_INCLUDE_DIR 给出头目录，绕过 find_library 的探测。
        let re_lib = find_archive(&re_build, &["libre.a", "re-static.lib"])
            .expect("libre archive not found before baresip configure");
        let mut cmd = Command::new("cmake");
        cmd.args([
            "-S",
            baresip_src.to_str().unwrap(),
            "-B",
            baresip_build.to_str().unwrap(),
            &format!("-DCMAKE_PREFIX_PATH={}", re_build.to_str().unwrap()),
            &format!("-Dre_DIR={}", re_cmake_dir.to_str().unwrap()),
            &format!("-DRE_LIBRARY={}", re_lib.to_str().unwrap()),
            &format!(
                "-DRE_INCLUDE_DIR={}",
                re_src.join("include").to_str().unwrap()
            ),
            "-DSTATIC=ON",
            "-DCMAKE_BUILD_TYPE=Release",
            &format!("-DMODULES={modules}"),
        ]);
        for a in cmake_cross_args() {
            cmd.arg(a);
        }
        // 同 re：Windows 上强制 cmake 选 OpenSSL 静态归档，避免 DLL 依赖。
        if target_os == "windows" {
            cmd.arg("-DOPENSSL_USE_STATIC_LIBS=ON");
        }
        run(&mut cmd);
        run(Command::new("cmake").args([
            "--build",
            baresip_build.to_str().unwrap(),
            "-j",
            &nproc.to_string(),
        ]));
    }

    // 3) link static libs
    // 先发现 cmake 实际产出的归档文件名与位置：MSVC 多配置生成器会把归档放在
    // <CONFIG> 子目录并命名为 libbaresip.lib / re-static.lib；其余平台为
    // libre.a / libbaresip.a。据此 emit 对应 parent 作为 native 搜索路径，并用
    // link_name() 还原出 rustc 能匹配的真实库名，彻底避免「could not find
    // native static library」或链接到错误归档。
    let re_lib = find_archive(&re_build, &["libre.a", "re-static.lib"])
        .expect("libre archive not found after build");
    let baresip_lib = find_archive(&baresip_build, &["libbaresip.a", "libbaresip.lib"])
        .expect("libbaresip archive not found after build");

    for dir in [re_lib.parent().unwrap(), baresip_lib.parent().unwrap()] {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    println!(
        "cargo:rustc-link-lib=static={}",
        link_name(&re_lib, &target_os)
    );
    println!(
        "cargo:rustc-link-lib=static={}",
        link_name(&baresip_lib, &target_os)
    );

    // 系统库与 OpenSSL/zlib：
    // - pthread/dl/m 等属 libc，无静态归档，保持动态（libc 必存在于目标机）。
    // - OpenSSL/zlib 在 Linux/macOS 静态链接（OPENSSL_STATIC/ZLIB_STATIC 已设，
    //   pkg-config 据此 emit -l=static），使二进制自包含。
    // - Windows 上 OpenSSL 由 choco 提供，按动态链接（见下方），运行时随 exe
    //   附带 DLL 即可自包含。
    if target_os == "windows" {
        // Windows 系统库：libc 等价物，始终存在于目标机，保持动态链接。静态 OpenSSL
        // 的归档会引用其中若干（ws2_32/crypt32/advapi32/user32/gdi32/bcrypt/secur32/
        // ole32/shell32/oleaut32/uuid/ncrypt/userenv/rpcrt4 等）；多列无副作用
        // （MSVC 会丢弃未引用的库），缺列则会 LNK2019。
        for lib in [
            "ws2_32", "wsock32", "crypt32", "gdi32", "winmm", "iphlpapi", "dbghelp", "bcrypt",
            "qwave", "user32", "advapi32", "secur32", "ole32", "shell32", "oleaut32", "uuid",
            "ncrypt", "userenv", "rpcrt4",
        ] {
            println!("cargo:rustc-link-lib={lib}");
        }
        // OpenSSL 静态链接：slproweb/choco 自带 `libssl_static.lib`/`libcrypto_static.lib`
        // （用 /MD 动态 CRT 编译），与 rustc 默认 /MD 兼容——链接进 exe 后二进制
        // 自包含，无需目标机安装、也无需同目录附带 libssl/libcrypto DLL。re/baresip
        // 在 Windows 上不依赖 zlib（与 baresip 官方 Windows CI 一致，仅装 openssl），
        // 故此处不链 z。
        if let Ok(openssl_lib) = std::env::var("OPENSSL_LIB_DIR") {
            if !openssl_lib.is_empty() {
                println!("cargo:rustc-link-search=native={openssl_lib}");
                // 优先静态导入库 lib<base>_static.lib（/MD 版），退回 lib<base>.lib。
                for base in ["ssl", "crypto"] {
                    let name = windows_import_lib_name(&openssl_lib, base);
                    println!("cargo:rustc-link-lib=static={name}");
                }
            }
        }
    } else {
        // OpenSSL：优先用 pkg-config 探测并 emit 正确的 -L 与 -l=static；
        // 若 pkg-config 不可用或探测失败（例如 macOS 上源码构建的 x86_64 openssl，
        // 其 .pc 在交叉环境下的 --static 探测可能失败），则回退到按
        // OPENSSL_LIB_DIR 直接链接静态 ssl/crypto，保证二进制自包含。
        let mut linked_openssl = false;
        if pkg_config::Config::new()
            .statik(true)
            .probe("openssl")
            .map_err(|e| eprintln!("pkg-config openssl: {e} (falling back to OPENSSL_LIB_DIR)"))
            .is_ok()
        {
            linked_openssl = true;
        }
        if !linked_openssl {
            if let Ok(openssl_lib) = std::env::var("OPENSSL_LIB_DIR") {
                println!("cargo:rustc-link-search=native={openssl_lib}");
                println!("cargo:rustc-link-lib=static=ssl");
                println!("cargo:rustc-link-lib=static=crypto");
                linked_openssl = true;
            }
        }
        if !linked_openssl {
            // 最后兜底：让 rustc 自行按默认搜索路径链接 ssl/crypto。
            println!("cargo:rustc-link-lib=ssl");
            println!("cargo:rustc-link-lib=crypto");
        }
        // macOS 默认无 zlib.pc，但系统自带动态 libz，直接按动态链接。
        if pkg_config::Config::new()
            .statik(true)
            .probe("zlib")
            .is_err()
        {
            println!("cargo:rustc-link-lib=z");
        }
        for lib in ["pthread", "dl", "m"] {
            println!("cargo:rustc-link-lib={lib}");
        }
        if target_os == "macos" {
            // libre 在 macOS 上解析 DNS 用到 libresolv。
            println!("cargo:rustc-link-lib=resolv");
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
    let mut bindings = bindgen::Builder::default()
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
        .generate_comments(false);

    // Windows：libclang 不预定义 WIN32（只预定义 _WIN32），而 libre 的 re_sa.h
    // 用 `#if defined(WIN32)` 决定走 winsock2.h 还是 POSIX 分支。不声明 WIN32 会
    // 误走 POSIX 分支去 include <sys/socket.h>（Windows 上不存在）。显式声明
    // -DWIN32 让头文件走 winsock2.h 分支（与 MSVC 编译 re/baresip 一致）。
    // libclang 找不到 Windows SDK 默认 include 路径，故把 SDK 的 um/shared/ucrt
    // 目录以 -isystem 加入，确保 winsock2.h 等系统头可解析。
    if target_os == "windows" {
        bindings = bindings.clang_arg("-DWIN32");
        for inc in windows_sdk_includes() {
            bindings = bindings.clang_arg(format!("-isystem{inc}"));
        }
    }

    let bindings = bindings
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

/// 探测 Windows SDK 的 C 头文件 include 目录（um / shared / ucrt），供 bindgen
/// 在 Windows 上解析 winsock2.h 等系统头。
///
/// 优先用 `WindowsSdkDir` 环境变量指向的 SDK 根，否则回退到
/// `C:\Program Files (x86)\Windows Kits\10`。SDK 版本目录按名称排序取最新。
fn windows_sdk_includes() -> Vec<String> {
    let sdk_root = std::env::var("WindowsSdkDir")
        .unwrap_or_else(|_| r"C:\Program Files (x86)\Windows Kits\10".to_string());
    let include_root = Path::new(&sdk_root).join("Include");
    let mut versions: Vec<String> = std::fs::read_dir(&include_root)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|v| {
            v.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        })
        .collect();
    if versions.is_empty() {
        return Vec::new();
    }
    versions.sort();
    let version = versions.pop().unwrap();
    let base = include_root.join(version);
    ["um", "shared", "ucrt"]
        .iter()
        .map(|sub| base.join(sub).to_string_lossy().into_owned())
        .collect()
}

/// 在 `dir` 下（含一层 <CONFIG> 子目录，兼容 MSVC 多配置生成器）递归查找
/// 任一候选静态归档名，返回第一个命中的绝对路径。
fn find_archive(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    if dir.is_dir() {
        for name in names {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let sub = entry.path();
            if sub.is_dir() {
                for name in names {
                    let candidate = sub.join(name);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    None
}

/// 把静态归档文件名转成传给 `rustc-link-lib=static=<name>` 的库名：
/// - 非 Windows：去掉 `lib` 前缀与 `.a` 后缀（libbaresip.a → baresip）；
/// - Windows：去掉 `.lib` 后缀（libbaresip.lib → libbaresip）。
fn link_name(archive: &Path, target_os: &str) -> String {
    let stem = archive
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if target_os == "windows" {
        stem.strip_suffix(".lib").unwrap_or(stem).to_string()
    } else {
        let no_lib = stem.strip_prefix("lib").unwrap_or(stem);
        no_lib.strip_suffix(".a").unwrap_or(no_lib).to_string()
    }
}

/// 在 Windows 上按 `OPENSSL_LIB_DIR` 中实际存在的导入库文件名，推断传给
/// `rustc-link-lib=static=<name>` 的库名 stem。slproweb 同时提供：
/// - 静态（`/MD` 编译）：libssl_static.lib / libcrypto_static.lib；
/// - 动态导入：ssl.lib / crypto.lib（3.x）或 libssl.lib / libcrypto.lib（4.x）。
///
/// 优先返回静态 variant，使 OpenSSL 被链进 exe、二进制自包含、不依赖 DLL。
fn windows_import_lib_name(openssl_lib_dir: &str, base: &str) -> String {
    let dir = Path::new(openssl_lib_dir);
    let static_with_lib = dir.join(format!("lib{base}_static.lib"));
    if static_with_lib.exists() {
        return format!("lib{base}_static");
    }
    let with_lib = dir.join(format!("lib{base}.lib"));
    if with_lib.exists() {
        return format!("lib{base}");
    }
    base.to_string()
}

/// 为目标架构交叉编译 re/baresip 时，给 cmake 传递对应工具链参数。
///
/// CI 现已为各架构选用原生 runner（linux-arm64 → ubuntu-24.04-arm、
/// mac-amd64 → macos-13），故常规构建下 TARGET == HOST，本函数返回空；
/// 仅当真正交叉（如本地手动交叉）时才输出工具链参数。
fn cmake_cross_args() -> Vec<String> {
    let target = std::env::var("TARGET").unwrap_or_default();
    let host = std::env::var("HOST").unwrap_or_default();
    if target.is_empty() || target == host {
        return Vec::new();
    }
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let mut args: Vec<String> = Vec::new();

    if os == "macos" {
        let apple = match arch.as_str() {
            "aarch64" => "arm64",
            other => other,
        };
        args.push(format!("-DCMAKE_OSX_ARCHITECTURES={apple}"));
    } else if os == "linux" {
        let upper = target.to_uppercase().replace(['-', '.'], "_");
        if let Ok(cc) = std::env::var(format!("CARGO_TARGET_{upper}_LINKER")) {
            args.push(format!("-DCMAKE_C_COMPILER={cc}"));
        }
        if let Ok(ar) = std::env::var(format!("CARGO_TARGET_{upper}_AR")) {
            args.push(format!("-DCMAKE_AR={ar}"));
        }
        args.push("-DCMAKE_SYSTEM_NAME=Linux".to_string());
        args.push(format!("-DCMAKE_SYSTEM_PROCESSOR={arch}"));
    }
    args
}
