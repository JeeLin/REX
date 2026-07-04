use anyhow::{bail, Result};
use async_trait::async_trait;
use bytes::{Buf, BytesMut};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

use crate::resp::RedisValue;

// ── 数据模型 ─────────────────────────────────────────────

/// Redis 连接响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisResponse {
    pub value: RedisValue,
    pub elapsed_ms: u64,
}

/// Redis 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    #[serde(default)]
    pub db: u8,
    pub name: Option<String>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 6379,
            password: None,
            db: 0,
            name: None,
        }
    }
}

// ── RedisConnector trait ─────────────────────────────────

#[async_trait]
pub trait RedisConnector: Send + Sync {
    /// 连接到 Redis 服务器
    async fn connect(&mut self) -> Result<()>;

    /// 执行 Redis 命令
    async fn execute(&mut self, command: &str) -> Result<RedisResponse>;

    /// 获取服务器信息
    async fn info(&mut self) -> Result<HashMap<String, String>>;

    /// 关闭连接
    async fn close(&mut self) -> Result<()>;
}

// ── RedisConnector TCP 实现 ────────────────────────────────

/// Redis 连接器（真实 TCP 实现）
pub struct RedisConnectorImpl {
    config: RedisConfig,
    stream: Option<TcpStream>,
}

impl RedisConnectorImpl {
    pub fn new(config: RedisConfig) -> Self {
        Self {
            config,
            stream: None,
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: RedisConfig = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }

    pub fn config(&self) -> &RedisConfig {
        &self.config
    }

    pub fn into_config(self) -> RedisConfig {
        self.config
    }

    /// 将命令编码为 RESP 格式并发送
    async fn send_raw(&mut self, command: &str) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let parts: Vec<&str> = command.split_whitespace().collect();
        let mut buf = format!("*{}\r\n", parts.len());
        for part in &parts {
            buf.push_str(&format!("${}\r\n{}\r\n", part.len(), part));
        }
        stream.write_all(buf.as_bytes()).await?;
        Ok(())
    }

    /// 读取一个完整的 RESP 响应
    async fn read_response(&mut self) -> Result<RedisValue> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;
        let mut buf = BytesMut::with_capacity(4096);
        let mut temp = [0u8; 4096];

        loop {
            if !buf.is_empty() {
                if let Some(val) = decode_resp(&mut buf)? {
                    return Ok(val);
                }
            }
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                bail!("connection closed by server");
            }
            buf.extend_from_slice(&temp[..n]);
        }
    }

    /// 发送命令并等待响应
    async fn send_command(&mut self, command: &str) -> Result<RedisValue> {
        self.send_raw(command).await?;
        self.read_response().await
    }
}

// ── 手动 RESP 解码 ───────────────────────────────────────

fn decode_resp(buf: &mut BytesMut) -> Result<Option<RedisValue>> {
    if buf.is_empty() {
        return Ok(None);
    }
    let prefix = buf[0] as char;

    let line_end = match find_crlf(buf) {
        Some(pos) => pos,
        None => return Ok(None),
    };
    let line = String::from_utf8_lossy(&buf[1..line_end]).to_string();
    buf.advance(line_end + 2);

    match prefix {
        '+' => Ok(Some(RedisValue::Status(line))),
        '-' => Ok(Some(RedisValue::Error(line))),
        ':' => {
            let n: i64 = line
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid integer: {line}"))?;
            Ok(Some(RedisValue::Integer(n)))
        }
        '$' => {
            let len: i64 = line
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid bulk length: {line}"))?;
            if len < 0 {
                Ok(Some(RedisValue::Bulk(None)))
            } else {
                let len = len as usize;
                if buf.len() < len + 2 {
                    buf.reserve(len + 2 - buf.len());
                    return Ok(None);
                }
                let data = String::from_utf8_lossy(&buf[..len]).to_string();
                buf.advance(len + 2);
                Ok(Some(RedisValue::Bulk(Some(data))))
            }
        }
        '*' => {
            let count: i64 = line
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid array count: {line}"))?;
            if count < 0 {
                Ok(Some(RedisValue::Null))
            } else {
                let count = count as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    match decode_resp(buf)? {
                        Some(v) => items.push(v),
                        None => return Ok(None),
                    }
                }
                Ok(Some(RedisValue::Array(items)))
            }
        }
        _ => Err(anyhow::anyhow!("unknown RESP prefix: {prefix}")),
    }
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    for i in 0..buf.len().saturating_sub(1) {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            return Some(i);
        }
    }
    None
}

// ── RedisConnector trait 实现 ─────────────────────────────

#[async_trait]
impl RedisConnector for RedisConnectorImpl {
    async fn connect(&mut self) -> Result<()> {
        info!(
            host = %self.config.host,
            port = self.config.port,
            db = self.config.db,
            "connecting to Redis"
        );

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let tcp = TcpStream::connect(&addr).await?;
        self.stream = Some(tcp);

        // AUTH if password is set
        if let Some(ref password) = self.config.password {
            let auth_cmd = format!("AUTH {password}");
            match self.send_command(&auth_cmd).await? {
                RedisValue::Status(s) if s == "OK" => {}
                RedisValue::Error(e) => bail!("AUTH failed: {e}"),
                other => bail!("AUTH unexpected response: {other:?}"),
            }
        }

        // SELECT database
        if self.config.db > 0 {
            let select_cmd = format!("SELECT {}", self.config.db);
            match self.send_command(&select_cmd).await? {
                RedisValue::Status(s) if s == "OK" => {}
                RedisValue::Error(e) => bail!("SELECT failed: {e}"),
                other => bail!("SELECT unexpected response: {other:?}"),
            }
        }

        info!(
            "connected to Redis {}:{}",
            self.config.host, self.config.port
        );
        Ok(())
    }

    async fn execute(&mut self, command: &str) -> Result<RedisResponse> {
        let start = std::time::Instant::now();
        let value = self.send_command(command).await?;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        Ok(RedisResponse { value, elapsed_ms })
    }

    async fn info(&mut self) -> Result<HashMap<String, String>> {
        let value = self.send_command("INFO server").await?;
        let mut info = HashMap::new();
        if let RedisValue::Bulk(Some(text)) = value {
            for line in text.lines() {
                if let Some((key, val)) = line.split_once(':') {
                    info.insert(key.to_string(), val.to_string());
                }
            }
        }
        Ok(info)
    }

    async fn close(&mut self) -> Result<()> {
        info!("closing Redis connection");
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.shutdown().await;
        }
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 6379);
        assert!(config.password.is_none());
        assert_eq!(config.db, 0);
    }

    #[test]
    fn redis_config_deserializes() {
        let json = r#"{"host":"10.0.0.1","port":6380,"password":"secret","db":2,"name":"cache"}"#;
        let config: RedisConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.host, "10.0.0.1");
        assert_eq!(config.port, 6380);
        assert_eq!(config.password, Some("secret".into()));
        assert_eq!(config.db, 2);
        assert_eq!(config.name, Some("cache".into()));
    }

    #[test]
    fn redis_config_optional_fields() {
        let json = r#"{"host":"localhost","port":6379}"#;
        let config: RedisConfig = serde_json::from_str(json).unwrap();
        assert!(config.password.is_none());
        assert_eq!(config.db, 0);
        assert!(config.name.is_none());
    }

    #[test]
    fn redis_connector_from_json() {
        let json = r#"{"host":"localhost","port":6379,"password":null,"db":0,"name":null}"#;
        let connector = RedisConnectorImpl::from_json(json).unwrap();
        assert_eq!(connector.config().host, "localhost");
        assert!(connector.stream.is_none());
    }

    #[test]
    fn redis_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn RedisConnector) {}
    }

    #[test]
    fn redis_response_serializes() {
        let resp = RedisResponse {
            value: RedisValue::Bulk(Some("hello".into())),
            elapsed_ms: 2,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Bulk"));
        assert!(json.contains("hello"));
        assert!(json.contains("elapsed_ms"));
    }

    #[test]
    fn decode_resp_status() {
        let mut buf = BytesMut::from(&b"+OK\r\n"[..]);
        let val = decode_resp(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Status("OK".into()));
    }

    #[test]
    fn decode_resp_error() {
        let mut buf = BytesMut::from(&b"-ERR unknown\r\n"[..]);
        let val = decode_resp(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Error("ERR unknown".into()));
    }

    #[test]
    fn decode_resp_integer() {
        let mut buf = BytesMut::from(&b":1000\r\n"[..]);
        let val = decode_resp(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Integer(1000));
    }

    #[test]
    fn decode_resp_bulk() {
        let mut buf = BytesMut::from(&b"$6\r\nfoobar\r\n"[..]);
        let val = decode_resp(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Bulk(Some("foobar".into())));
    }

    #[test]
    fn decode_resp_bulk_null() {
        let mut buf = BytesMut::from(&b"$-1\r\n"[..]);
        let val = decode_resp(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Bulk(None));
    }

    #[test]
    fn decode_resp_array() {
        let mut buf = BytesMut::from(&b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"[..]);
        let val = decode_resp(&mut buf).unwrap().unwrap();
        assert_eq!(
            val,
            RedisValue::Array(vec![
                RedisValue::Bulk(Some("foo".into())),
                RedisValue::Bulk(Some("bar".into())),
            ])
        );
    }

    #[test]
    fn decode_resp_empty_array() {
        let mut buf = BytesMut::from(&b"*0\r\n"[..]);
        let val = decode_resp(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Array(vec![]));
    }

    #[test]
    fn decode_resp_incomplete() {
        let mut buf = BytesMut::from(&b"+OK"[..]);
        let val = decode_resp(&mut buf).unwrap();
        assert!(val.is_none());
    }
}
