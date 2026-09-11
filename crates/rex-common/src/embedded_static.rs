//! 嵌入式静态文件服务 — 将前端 dist 目录编译进二进制，保留单文件分发。
//!
//! 通过 `embedded-static` feature 启用，Hub 和 Agent 共享同一实现。

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{header, Request, Response, StatusCode};
use bytes::Bytes;
use include_dir::Dir;
use tower::Service;

/// Tower Service：从编译期嵌入的前端 dist 目录提供静态文件。
///
/// 使用方式：
/// ```ignore
/// use include_dir::include_dir;
/// use rex_common::embedded_static::EmbeddedStatic;
///
/// static DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../packages/rex-console-web/dist");
/// let embedded = EmbeddedStatic::new("/", &DIST);
/// ```
#[derive(Clone)]
pub struct EmbeddedStatic<'a> {
    prefix: &'static str,
    dist: &'a Dir<'a>,
}

impl<'a> EmbeddedStatic<'a> {
    pub fn new(prefix: &'static str, dist: &'a Dir<'a>) -> Self {
        Self { prefix, dist }
    }

    /// 根据请求路径查找嵌入文件并返回 HTTP 响应。
    fn serve(&self, req: &Request<Body>) -> Option<Response<Body>> {
        let path = req.uri().path();

        // 去掉 prefix 前缀
        let rel = if let Some(rest) = path.strip_prefix(self.prefix) {
            rest
        } else {
            path
        };

        // 安全：拒绝路径遍历
        if rel.contains("..") {
            return None;
        }

        let file_path = rel.trim_start_matches('/');
        let file_path = if file_path.is_empty() {
            "index.html"
        } else {
            file_path
        };

        // Track whether we fell back to index.html for SPA routing
        let mut used_fallback = false;

        let file = self.dist.get_file(file_path).or_else(|| {
            // SPA fallback：非文件请求 → index.html
            if !file_path.contains('.') {
                used_fallback = true;
                self.dist.get_file("index.html")
            } else {
                None
            }
        })?;

        // When SPA fallback serves index.html, always use text/html
        // regardless of the original request path's extension.
        let mime = if used_fallback {
            "text/html".to_string()
        } else {
            mime_guess::from_path(file_path)
                .first_or_octet_stream()
                .to_string()
        };
        let body = Body::from(Bytes::copy_from_slice(file.contents()));
        let resp = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(body)
            .ok()?;

        Some(resp)
    }
}

impl<'a> Service<Request<Body>> for EmbeddedStatic<'a> {
    type Response = Response<Body>;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let resp = self.serve(&req).unwrap_or_else(|| {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap()
        });
        Box::pin(async { Ok(resp) })
    }
}
