//! 嵌入式静态文件服务 — 将前端 dist 目录编译进二进制，保留单文件分发。

use include_dir::{include_dir, Dir};
use rex_common::embedded_static::EmbeddedStatic;

static DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../packages/rex-console-web/dist");

/// Tower Service：从编译期嵌入的前端 dist 目录提供静态文件。
pub fn create_embedded_static(prefix: &'static str) -> EmbeddedStatic<'static> {
    EmbeddedStatic::new(prefix, &DIST)
}
