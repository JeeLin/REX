use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

// ── Redis 值类型 ─────────────────────────────────────────

/// RESP 协议中的 Redis 值类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum RedisValue {
    /// +OK\r\n 状态响应
    Status(String),
    /// -ERR message\r\n 错误响应
    Error(String),
    /// :1000\r\n 整数响应
    Integer(i64),
    /// $6\r\nfoobar\r\n 批量字符串
    Bulk(Option<String>),
    /// *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n 数组
    Array(Vec<RedisValue>),
    /// $-1\r\n 空值
    Null,
}

use serde::{Deserialize, Serialize};

impl RedisValue {
    /// 将 RedisValue 格式化为可读字符串
    pub fn display(&self) -> String {
        match self {
            RedisValue::Status(s) => s.clone(),
            RedisValue::Error(s) => format!("(error) {s}"),
            RedisValue::Integer(n) => format!("(integer) {n}"),
            RedisValue::Bulk(Some(s)) => format!("\"{s}\""),
            RedisValue::Bulk(None) => "(nil)".to_string(),
            RedisValue::Array(items) => {
                if items.is_empty() {
                    "(empty array)".to_string()
                } else {
                    items
                        .iter()
                        .enumerate()
                        .map(|(i, v)| format!("{}) {}", i + 1, v.display()))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            }
            RedisValue::Null => "(nil)".to_string(),
        }
    }
}

// ── RESP 编解码错误 ──────────────────────────────────────

#[derive(Debug)]
pub enum RespError {
    Io(std::io::Error),
    Invalid(String),
}

impl std::fmt::Display for RespError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RespError::Io(e) => write!(f, "IO error: {e}"),
            RespError::Invalid(s) => write!(f, "Invalid RESP: {s}"),
        }
    }
}

impl std::error::Error for RespError {}

impl From<std::io::Error> for RespError {
    fn from(e: std::io::Error) -> Self {
        RespError::Io(e)
    }
}

// ── RESP Decoder ─────────────────────────────────────────

/// RESP 协议解码器，将字节流解码为 RedisValue
pub struct RespDecoder;

impl RespDecoder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RespDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for RespDecoder {
    type Item = RedisValue;
    type Error = RespError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let line_end = find_crlf(src);
        let line_end = match line_end {
            Some(pos) => pos,
            None => return Ok(None),
        };

        let prefix = src[0] as char;
        let line = String::from_utf8_lossy(&src[1..line_end]).to_string();

        // 消耗行内容 + \r\n
        src.advance(line_end + 2);

        match prefix {
            '+' => Ok(Some(RedisValue::Status(line))),
            '-' => Ok(Some(RedisValue::Error(line))),
            ':' => {
                let n: i64 = line.parse().map_err(|_| RespError::Invalid(line.clone()))?;
                Ok(Some(RedisValue::Integer(n)))
            }
            '$' => {
                let len: i64 = line.parse().map_err(|_| RespError::Invalid(line.clone()))?;
                if len < 0 {
                    Ok(Some(RedisValue::Bulk(None)))
                } else {
                    let len = len as usize;
                    // 需要 len 字节 + \r\n
                    if src.len() < len + 2 {
                        src.reserve(len + 2 - src.len());
                        return Ok(None);
                    }
                    let data = String::from_utf8_lossy(&src[..len]).to_string();
                    src.advance(len + 2); // 跳过 \r\n
                    Ok(Some(RedisValue::Bulk(Some(data))))
                }
            }
            '*' => {
                let count: i64 = line.parse().map_err(|_| RespError::Invalid(line.clone()))?;
                if count < 0 {
                    Ok(Some(RedisValue::Null))
                } else {
                    let count = count as usize;
                    let mut items = Vec::with_capacity(count);
                    for _ in 0..count {
                        match Self::new().decode(src)? {
                            Some(v) => items.push(v),
                            None => return Ok(None), // 数据不完整
                        }
                    }
                    Ok(Some(RedisValue::Array(items)))
                }
            }
            _ => Err(RespError::Invalid(format!("unknown prefix: {prefix}"))),
        }
    }
}

// ── RESP Encoder ─────────────────────────────────────────

/// RESP 协议编码器，将命令字符串编码为 RESP 格式
pub struct RespEncoder;

impl RespEncoder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RespEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Encoder<String> for RespEncoder {
    type Error = RespError;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // 将命令按空格拆分为 RESP 数组
        let parts: Vec<&str> = item.split_whitespace().collect();
        let header = format!("*{}\r\n", parts.len());
        dst.extend_from_slice(header.as_bytes());
        for part in &parts {
            let len_line = format!("${}\r\n", part.len());
            dst.extend_from_slice(len_line.as_bytes());
            dst.extend_from_slice(part.as_bytes());
            dst.extend_from_slice(b"\r\n");
        }
        Ok(())
    }
}

// ── 工具函数 ─────────────────────────────────────────────

/// 在缓冲区中查找 \r\n 的位置
fn find_crlf(src: &[u8]) -> Option<usize> {
    for i in 0..src.len().saturating_sub(1) {
        if src[i] == b'\r' && src[i + 1] == b'\n' {
            return Some(i);
        }
    }
    None
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use tokio_util::codec::{Decoder, Encoder};

    // ── RedisValue::display ─────────────────────────────

    #[test]
    fn display_status() {
        assert_eq!(RedisValue::Status("OK".into()).display(), "OK");
    }

    #[test]
    fn display_error() {
        assert_eq!(
            RedisValue::Error("ERR unknown command".into()).display(),
            "(error) ERR unknown command"
        );
    }

    #[test]
    fn display_integer() {
        assert_eq!(RedisValue::Integer(42).display(), "(integer) 42");
    }

    #[test]
    fn display_bulk_some() {
        assert_eq!(
            RedisValue::Bulk(Some("hello".into())).display(),
            "\"hello\""
        );
    }

    #[test]
    fn display_bulk_none() {
        assert_eq!(RedisValue::Bulk(None).display(), "(nil)");
    }

    #[test]
    fn display_array_empty() {
        assert_eq!(RedisValue::Array(vec![]).display(), "(empty array)");
    }

    #[test]
    fn display_array_items() {
        let arr = RedisValue::Array(vec![
            RedisValue::Bulk(Some("name".into())),
            RedisValue::Bulk(Some("Alice".into())),
        ]);
        let d = arr.display();
        assert!(d.contains("1) \"name\""));
        assert!(d.contains("2) \"Alice\""));
    }

    #[test]
    fn display_null() {
        assert_eq!(RedisValue::Null.display(), "(nil)");
    }

    // ── RESP Decoder ────────────────────────────────────

    #[test]
    fn decode_status() {
        let mut buf = BytesMut::from(&b"+OK\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Status("OK".into()));
    }

    #[test]
    fn decode_error() {
        let mut buf = BytesMut::from(&b"-ERR unknown\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Error("ERR unknown".into()));
    }

    #[test]
    fn decode_integer() {
        let mut buf = BytesMut::from(&b":1000\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Integer(1000));
    }

    #[test]
    fn decode_integer_negative() {
        let mut buf = BytesMut::from(&b":-1\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Integer(-1));
    }

    #[test]
    fn decode_bulk_string() {
        let mut buf = BytesMut::from(&b"$6\r\nfoobar\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Bulk(Some("foobar".into())));
    }

    #[test]
    fn decode_bulk_null() {
        let mut buf = BytesMut::from(&b"$-1\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Bulk(None));
    }

    #[test]
    fn decode_array() {
        let mut buf = BytesMut::from(&b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(
            val,
            RedisValue::Array(vec![
                RedisValue::Bulk(Some("foo".into())),
                RedisValue::Bulk(Some("bar".into())),
            ])
        );
    }

    #[test]
    fn decode_empty_array() {
        let mut buf = BytesMut::from(&b"*0\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Array(vec![]));
    }

    #[test]
    fn decode_null_array() {
        let mut buf = BytesMut::from(&b"*-1\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(val, RedisValue::Null);
    }

    #[test]
    fn decode_incomplete_returns_none() {
        let mut buf = BytesMut::from(&b"+OK"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap();
        assert!(val.is_none());
    }

    #[test]
    fn decode_empty_returns_none() {
        let mut buf = BytesMut::new();
        let val = RespDecoder::new().decode(&mut buf).unwrap();
        assert!(val.is_none());
    }

    #[test]
    fn decode_nested_array() {
        // *2\r\n*2\r\n$1\r\na\r\n$1\r\nb\r\n:42\r\n
        let mut buf = BytesMut::from(&b"*2\r\n*2\r\n$1\r\na\r\n$1\r\nb\r\n:42\r\n"[..]);
        let val = RespDecoder::new().decode(&mut buf).unwrap().unwrap();
        assert_eq!(
            val,
            RedisValue::Array(vec![
                RedisValue::Array(vec![
                    RedisValue::Bulk(Some("a".into())),
                    RedisValue::Bulk(Some("b".into())),
                ]),
                RedisValue::Integer(42),
            ])
        );
    }

    // ── RESP Encoder ────────────────────────────────────

    #[test]
    fn encode_simple_command() {
        let mut buf = BytesMut::new();
        RespEncoder::new()
            .encode("GET key".into(), &mut buf)
            .unwrap();
        assert_eq!(&buf[..], b"*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn encode_single_word() {
        let mut buf = BytesMut::new();
        RespEncoder::new().encode("PING".into(), &mut buf).unwrap();
        assert_eq!(&buf[..], b"*1\r\n$4\r\nPING\r\n");
    }

    #[test]
    fn encode_with_spaces_in_value() {
        let mut buf = BytesMut::new();
        RespEncoder::new()
            .encode("SET key hello world".into(), &mut buf)
            .unwrap();
        assert_eq!(
            &buf[..],
            b"*4\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nhello\r\n$5\r\nworld\r\n"
        );
    }

    // ── RedisValue serialization ────────────────────────

    #[test]
    fn redis_value_json_roundtrip() {
        let val = RedisValue::Array(vec![
            RedisValue::Status("OK".into()),
            RedisValue::Integer(42),
            RedisValue::Bulk(Some("hello".into())),
            RedisValue::Bulk(None),
            RedisValue::Null,
        ]);
        let json = serde_json::to_string(&val).unwrap();
        let parsed: RedisValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, parsed);
    }
}
