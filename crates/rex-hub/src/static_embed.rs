//! 静态文件服务：编译嵌入（生产）或运行时目录（开发）。
//!
//! - `embedded-static` feature 启用：将前端 dist 编译进二进制（单文件分发）
//! - 未启用：从 `REX_STATIC_DIR` 环境变量指向的目录提供文件（开发模式）

use std::path::PathBuf;

/// 获取开发模式下的静态文件目录路径。
pub fn dev_static_dir() -> PathBuf {
    std::env::var("REX_STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/rex-console-web/dist")
        })
}

#[cfg(feature = "embedded-static")]
pub use embedded_impl::create_embedded_static;

#[cfg(feature = "embedded-static")]
mod embedded_impl {
    use include_dir::{include_dir, Dir};
    use rex_common::embedded_static::EmbeddedStatic;

    static DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../packages/rex-console-web/dist");

    pub fn create_embedded_static(prefix: &'static str) -> EmbeddedStatic<'static> {
        tracing::info!("serving static files from embedded binary (production mode)");
        EmbeddedStatic::new(prefix, &DIST)
    }
}
