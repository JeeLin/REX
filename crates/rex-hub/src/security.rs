use axum::body::Body;
use axum::http::{header, Request};
use axum::middleware::Next;
use axum::response::Response;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

/// 登录速率限制器：同一 IP 在时间窗口内最多允许 N 次失败登录
pub struct RateLimiter {
    attempts: std::sync::Mutex<HashMap<IpAddr, Vec<Instant>>>,
    max_attempts: usize,
    window: std::time::Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window: std::time::Duration) -> Self {
        Self {
            attempts: std::sync::Mutex::new(HashMap::new()),
            max_attempts,
            window,
        }
    }

    /// 检查 IP 是否被限流。返回 true 表示允许，false 表示应拒绝
    pub fn check(&self, ip: &IpAddr) -> bool {
        let mut attempts = self.attempts.lock().unwrap();
        let now = Instant::now();
        let timestamps = attempts.entry(*ip).or_default();

        // 清理过期记录
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        timestamps.len() < self.max_attempts
    }

    /// 记录一次失败登录
    pub fn record_failure(&self, ip: &IpAddr) {
        let mut attempts = self.attempts.lock().unwrap();
        let now = Instant::now();
        let timestamps = attempts.entry(*ip).or_default();

        // 清理过期记录
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        timestamps.push(now);
    }

    /// 登录成功时清理该 IP 的失败记录
    pub fn clear(&self, ip: &IpAddr) {
        let mut attempts = self.attempts.lock().unwrap();
        attempts.remove(ip);
    }
}

/// 安全头中间件：为所有 HTTP 响应添加安全头
pub async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // 防止 MIME 类型嗅探
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        header::HeaderValue::from_static("nosniff"),
    );

    // 防止点击劫持
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        header::HeaderValue::from_static("DENY"),
    );

    // CSP：允许 self + ws/wss（WebSocket）+ unsafe-inline（内联样式）
    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        header::HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ws: wss:; font-src 'self'",
        ),
    );

    // HSTS（仅 HTTPS 时有意义，但加上不会有害）
    headers.insert(
        header::HeaderName::from_static("strict-transport-security"),
        header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // 防止浏览器缓存敏感内容
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(3, std::time::Duration::from_secs(300));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        assert!(limiter.check(&ip)); // 0/3
        limiter.record_failure(&ip);
        assert!(limiter.check(&ip)); // 1/3
        limiter.record_failure(&ip);
        assert!(limiter.check(&ip)); // 2/3
        limiter.record_failure(&ip);
        assert!(!limiter.check(&ip)); // 3/3 → blocked
    }

    #[test]
    fn rate_limiter_clears_on_success() {
        let limiter = RateLimiter::new(2, std::time::Duration::from_secs(300));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        limiter.record_failure(&ip);
        limiter.record_failure(&ip);
        assert!(!limiter.check(&ip)); // 2/2 → blocked

        limiter.clear(&ip);
        assert!(limiter.check(&ip)); // cleared → allowed
    }

    #[test]
    fn rate_limiter_different_ips_independent() {
        let limiter = RateLimiter::new(1, std::time::Duration::from_secs(300));
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        limiter.record_failure(&ip1);
        assert!(!limiter.check(&ip1)); // ip1 blocked
        assert!(limiter.check(&ip2)); // ip2 still allowed
    }

    #[test]
    fn rate_limiter_window_expires() {
        let limiter = RateLimiter::new(2, std::time::Duration::from_millis(50));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        limiter.record_failure(&ip);
        limiter.record_failure(&ip);
        assert!(!limiter.check(&ip));

        // Wait for window to expire
        std::thread::sleep(std::time::Duration::from_millis(60));
        assert!(limiter.check(&ip)); // window expired → allowed
    }
}
