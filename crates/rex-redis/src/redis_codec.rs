//! Redis 值格式自动探测与解码。
//!
//! 支持格式：Msgpack、PHPSerialize、JavaSerialize、Pickle、zlib/gzip/zstd 压缩。

use flate2::read::{GzDecoder, ZlibDecoder};
use std::io::Read;

/// 检测到的格式名称（与前端 FormatViewer 标签对应）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedFormat {
    Text,
    Json,
    Msgpack,
    PhpSerialize,
    JavaSerialize,
    Pickle,
    Zlib,
    Gzip,
    Zstd,
    Binary,
}

impl DetectedFormat {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Msgpack => "msgpack",
            Self::PhpSerialize => "php_serialize",
            Self::JavaSerialize => "java_serialize",
            Self::Pickle => "pickle",
            Self::Zlib => "zlib",
            Self::Gzip => "gzip",
            Self::Zstd => "zstd",
            Self::Binary => "binary",
        }
    }
}

/// 格式检测结果。
#[derive(Debug, Clone)]
pub struct FormatDetection {
    /// 检测到的格式。
    pub format: DetectedFormat,
    /// 解码后的可读文本（Msgpack/PHP/Java/Pickle 的 pretty-print，压缩的解压结果）。
    pub decoded: Option<String>,
    /// 压缩算法名称（仅压缩格式有值，如 "zlib"、"gzip"、"zstd"）。
    pub compression: Option<String>,
    /// 原始字节大小。
    pub raw_size: usize,
}

/// 对 `bytes` 进行格式自动探测并解码。
///
/// 检测流程：
/// 1. 压缩格式（zstd → gzip → zlib）→ 解压后递归检测（最多 1 层）
/// 2. 序列化格式（Msgpack → Pickle → PHP → Java）
/// 3. JSON
/// 4. 不可打印字符 → Binary
/// 5. 默认 Text
pub fn detect_and_decode(bytes: &[u8]) -> FormatDetection {
    let raw_size = bytes.len();

    // 1. 压缩格式检测
    if let Some(d) = detect_compressed(bytes, raw_size) {
        return d;
    }

    // 2. 序列化格式检测
    if let Some(d) = detect_serialized(bytes, raw_size) {
        return d;
    }

    // 3. JSON
    if let Ok(s) = std::str::from_utf8(bytes) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(s) {
            let pretty = serde_json::to_string_pretty(&val).unwrap_or_default();
            return FormatDetection {
                format: DetectedFormat::Json,
                decoded: Some(pretty),
                compression: None,
                raw_size,
            };
        }
    }

    // 4. 不可打印字符 → Binary
    if bytes
        .iter()
        .any(|&b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r')
    {
        return FormatDetection {
            format: DetectedFormat::Binary,
            decoded: None,
            compression: None,
            raw_size,
        };
    }

    // 5. 默认 Text
    FormatDetection {
        format: DetectedFormat::Text,
        decoded: None,
        compression: None,
        raw_size,
    }
}

// ---------------------------------------------------------------------------
// 压缩格式检测
// ---------------------------------------------------------------------------

fn detect_compressed(bytes: &[u8], raw_size: usize) -> Option<FormatDetection> {
    // zstd: magic 0x28B52FFD
    if bytes.len() >= 4
        && bytes[0] == 0x28
        && bytes[1] == 0xB5
        && bytes[2] == 0x2F
        && bytes[3] == 0xFD
    {
        if let Ok(decompressed) = zstd::decode_all(bytes) {
            let inner = detect_and_decode(&decompressed);
            return Some(FormatDetection {
                format: inner.format,
                decoded: inner.decoded,
                compression: Some("zstd".into()),
                raw_size,
            });
        }
    }

    // gzip: magic 0x1F 0x8B
    if bytes.len() >= 2 && bytes[0] == 0x1F && bytes[1] == 0x8B {
        let mut decoder = GzDecoder::new(bytes);
        let mut buf = Vec::new();
        if decoder.read_to_end(&mut buf).is_ok() {
            let inner = detect_and_decode(&buf);
            return Some(FormatDetection {
                format: inner.format,
                decoded: inner.decoded,
                compression: Some("gzip".into()),
                raw_size,
            });
        }
    }

    // zlib: CMF byte 0x78 (deflate, window size 32k) — zlib 容器需 ZlibDecoder
    if !bytes.is_empty() && bytes[0] == 0x78 {
        let mut decoder = ZlibDecoder::new(bytes);
        let mut buf = Vec::new();
        if decoder.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
            let inner = detect_and_decode(&buf);
            return Some(FormatDetection {
                format: inner.format,
                decoded: inner.decoded,
                compression: Some("zlib".into()),
                raw_size,
            });
        }
    }

    None
}

// ---------------------------------------------------------------------------
// 序列化格式检测
// ---------------------------------------------------------------------------

fn detect_serialized(bytes: &[u8], raw_size: usize) -> Option<FormatDetection> {
    // Msgpack: 通过 rmpv 尝试解码
    if let Some(d) = try_msgpack(bytes, raw_size) {
        return Some(d);
    }
    // Pickle: 首字节 0x80 (protocol) 或 0x28 '(' (OP: PROTO)
    if let Some(d) = try_pickle(bytes, raw_size) {
        return Some(d);
    }
    // PHPSerialize: 首字节为 a/O/s/i/b/N/d
    if let Some(d) = try_php_serialize(bytes, raw_size) {
        return Some(d);
    }
    // JavaSerialize: 首 2 字节 0xAC 0xED
    if let Some(d) = try_java_serialize(bytes, raw_size) {
        return Some(d);
    }
    None
}

// ---------------------------------------------------------------------------
// Msgpack
// ---------------------------------------------------------------------------

fn try_msgpack(bytes: &[u8], raw_size: usize) -> Option<FormatDetection> {
    // Msgpack 格式检查：首字节必须是合法的 msgpack type marker
    let is_msgpack_header = matches!(
        bytes.first()?,
        0x00..=0x7f | // positive fixint
        0x80..=0x8f | // fixmap
        0x90..=0x9f | // fixarray
        0xa0..=0xbf | // fixstr
        0xc0..=0xc3 | // nil, false, true
        0xc4..=0xc6 | // bin 8/16/32
        0xc7..=0xc9 | // ext 8/16/32
        0xca..=0xcb | // float 32/64
        0xcc..=0xcf | // uint 8/16/32/64
        0xd0..=0xd3 | // int 8/16/32/64
        0xd4..=0xd8 | // fixext 1/2/4/8/16
        0xe0..=0xff   // negative fixint
    );

    if !is_msgpack_header {
        return None;
    }

    let mut cursor = bytes;
    let value = rmpv::decode::read_value(&mut cursor).ok()?;

    // 必须完全消费所有字节，否则可能是普通文本碰巧匹配首字节
    if !cursor.is_empty() {
        return None;
    }

    // 太简单的值（单个整数/nil/bool）不作为 msgpack 检测结果
    // 避免将普通文本误判为 msgpack
    match &value {
        rmpv::Value::Nil
        | rmpv::Value::Boolean(_)
        | rmpv::Value::Integer(_)
            // 单个简单值只在字节长度 ≤ 5 时才认为是 msgpack（fixint/fixed 长度）
            // 长字节序列几乎不可能是纯 msgpack 简单值
            if bytes.len() > 5 => {
                return None;
            }
        _ => {}
    }

    let pretty = format!("{:#?}", value);
    Some(FormatDetection {
        format: DetectedFormat::Msgpack,
        decoded: Some(pretty),
        compression: None,
        raw_size,
    })
}

// ---------------------------------------------------------------------------
// Pickle
// ---------------------------------------------------------------------------

fn try_pickle(bytes: &[u8], raw_size: usize) -> Option<FormatDetection> {
    let first = *bytes.first()?;

    // 判定需「结构性」约束，避免与正常文本误判：
    // - 高字节 opcode（0x80/0x8c/0x8d/0x8e）不是合法 UTF-8 首字节，基本可确定是 pickle 帧
    // - 0x43/0x42/0x62 等 ASCII 字母 opcode 过于常见（如 `Compress…`、`b:1;`），
    //   必须校验后续长度字段落在缓冲内，否则只是普通文本
    // - 0x63 GLOBAL 必须后随 `module\nclass\n` 结构
    // - 0x28 MARK（`(`）过于常见，不再作为单独判据，避免误报
    let looks_like_pickle = match first {
        0x80 | 0x8c | 0x8d | 0x8e => true, // PROTO / SHORT_BINUNICODE* / SHORT_BINBYTES8（高字节，非 UTF-8 首字节）
        0x43 | 0x42 => {
            // BINBYTES(proto1, 4B LE len) / BINPERSID(4B LE len)：长度须落在缓冲内
            bytes.len() >= 5 && {
                let n = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
                5 + n <= bytes.len()
            }
        }
        0x62 => {
            // BINBYTES(proto0/1, 1B len)：长度须落在缓冲内（排除 `b:1;` 之类文本）
            bytes.len() >= 2 && {
                let n = bytes[1] as usize;
                2 + n <= bytes.len()
            }
        }
        0x63 => {
            // GLOBAL：必须后随 `module\nclass\n`（排除 `compress me` 之类文本）
            match bytes.get(1..) {
                Some(rest) => match std::str::from_utf8(rest) {
                    Ok(s) => match s.find('\n') {
                        Some(e1) if e1 > 0 => {
                            let class_part = &s[e1 + 1..];
                            !class_part.is_empty() && class_part.find('\n').is_some()
                        }
                        _ => false,
                    },
                    Err(_) => false,
                },
                None => false,
            }
        }
        _ => false,
    };

    if !looks_like_pickle {
        return None;
    }

    // 尝试用 Python pickle-like 解码：提取字符串和结构
    let decoded = decode_pickle_simple(bytes);
    Some(FormatDetection {
        format: DetectedFormat::Pickle,
        decoded: Some(decoded),
        compression: None,
        raw_size,
    })
}

/// 简易 Pickle 解码：提取可读字符串和基本结构标记。
fn decode_pickle_simple(bytes: &[u8]) -> String {
    let mut result = String::from("[Pickle] ");
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            // SHORT_BINUNICODE (protocol 4): 0x8c + 1 byte len + data
            0x8c if i + 1 < bytes.len() => {
                let len = bytes[i + 1] as usize;
                if i + 2 + len <= bytes.len() {
                    if let Ok(s) = std::str::from_utf8(&bytes[i + 2..i + 2 + len]) {
                        result.push_str(&format!("\"{}\" ", s));
                    }
                    i += 2 + len;
                    continue;
                }
            }
            // BINBYTES (protocol 1): 0x42 + 4 byte len + data
            0x42 if i + 4 < bytes.len() => {
                let len =
                    u32::from_le_bytes([bytes[i + 1], bytes[i + 2], bytes[i + 3], bytes[i + 4]])
                        as usize;
                if i + 5 + len <= bytes.len() {
                    if let Ok(s) = std::str::from_utf8(&bytes[i + 5..i + 5 + len]) {
                        result.push_str(&format!("bytes(\"{}\") ", s));
                    }
                    i += 5 + len;
                    continue;
                }
            }
            // GLOBAL: 0x63 + module\n class\n
            0x63 => {
                if let Some(rest) = bytes.get(i + 1..) {
                    if let Ok(s) = std::str::from_utf8(rest) {
                        if let Some(end) = s.find('\n') {
                            let module = &s[..end];
                            let rest2 = &s[end + 1..];
                            if let Some(end2) = rest2.find('\n') {
                                let class = &rest2[..end2];
                                result.push_str(&format!("global({} {}) ", module, class));
                                i += 1 + end + 1 + end2 + 1;
                                continue;
                            }
                        }
                    }
                }
            }
            // MARK: 0x28
            0x28 => {
                result.push_str("MARK ");
            }
            // STOP: 0x2e
            0x2e => {
                result.push_str("STOP ");
            }
            _ => {}
        }
        i += 1;
    }

    result
}

// ---------------------------------------------------------------------------
// PHPSerialize
// ---------------------------------------------------------------------------

fn try_php_serialize(bytes: &[u8], raw_size: usize) -> Option<FormatDetection> {
    let s = std::str::from_utf8(bytes).ok()?;
    let first = s.chars().next()?;

    // PHP serialize format starts with: a (array), O (object), s (string), i (int),
    // b (bool), N (null), d (double), r/R (reference)
    match first {
        'a' | 'O' | 's' | 'i' | 'b' | 'N' | 'd' | 'r' | 'R' => {}
        _ => return None,
    }

    // 验证基本结构
    if !is_valid_php_serialize(s) {
        return None;
    }

    let pretty = format_php_serialize(s);
    Some(FormatDetection {
        format: DetectedFormat::PhpSerialize,
        decoded: Some(pretty),
        compression: None,
        raw_size,
    })
}

fn is_valid_php_serialize(s: &str) -> bool {
    // 基本验证：匹配常见 PHP serialize 模式
    // a:N:{...}  O:N:"name":N:{...}  s:N:"...";  i:N;  b:N;  N;  d:N;
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    match bytes[0] {
        b'a' => {
            // a:N:{...}
            bytes.len() > 4 && bytes[1] == b':' && bytes[2].is_ascii_digit()
        }
        b'O' => {
            // O:N:"name":N:{...}
            bytes.len() > 6 && bytes[1] == b':'
        }
        b's' => {
            // s:N:"...";
            bytes.len() > 4 && bytes[1] == b':'
        }
        b'i' => {
            // i:N;
            bytes.len() > 3 && bytes[1] == b':'
        }
        b'b' => {
            // b:N;
            bytes.len() > 3 && bytes[1] == b':' && (bytes[2] == b'0' || bytes[2] == b'1')
        }
        b'N' => bytes.len() == 1 || bytes[1] == b';',
        b'd' => {
            // d:N;
            bytes.len() > 3 && bytes[1] == b':'
        }
        _ => false,
    }
}

fn format_php_serialize(s: &str) -> String {
    // 缩进格式化：用缩进表示嵌套
    let mut result = String::new();
    let mut depth = 0;
    let chars = s.chars();

    for c in chars {
        match c {
            '{' => {
                result.push_str(" {\n");
                depth += 1;
                result.push_str(&"  ".repeat(depth));
            }
            '}' => {
                depth = depth.saturating_sub(1);
                result.push('\n');
                result.push_str(&"  ".repeat(depth));
                result.push('}');
            }
            ';' => {
                result.push_str(";\n");
                if depth > 0 {
                    result.push_str(&"  ".repeat(depth));
                }
            }
            _ => result.push(c),
        }
    }

    result
}

// ---------------------------------------------------------------------------
// JavaSerialize
// ---------------------------------------------------------------------------

fn try_java_serialize(bytes: &[u8], raw_size: usize) -> Option<FormatDetection> {
    // Java 序列化 magic: 0xAC 0xED, version: 0x00 0x05
    if bytes.len() < 4 {
        return None;
    }
    if bytes[0] != 0xAC || bytes[1] != 0xED {
        return None;
    }

    let decoded = decode_java_serialize(bytes);
    Some(FormatDetection {
        format: DetectedFormat::JavaSerialize,
        decoded: Some(decoded),
        compression: None,
        raw_size,
    })
}

/// 简易 Java 序列化解码：提取类名和基本字段。
fn decode_java_serialize(bytes: &[u8]) -> String {
    let mut result = String::from("[Java Serialized] ");

    // 跳过 magic (2) + version (2) = 4 bytes
    let mut i = 4;
    let mut found_class = false;

    while i + 2 < bytes.len() {
        match bytes[i] {
            // TC_OBJECT = 0x73
            0x73 => {
                result.push_str("Object ");
                i += 1;
            }
            // TC_CLASSDESC = 0x72
            0x72 => {
                i += 1;
                if i + 1 < bytes.len() {
                    let name_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
                    i += 2;
                    if i + name_len <= bytes.len() {
                        if let Ok(name) = std::str::from_utf8(&bytes[i..i + name_len]) {
                            found_class = true;
                            result.push_str(&format!("class:{} ", name));
                        }
                        i += name_len;
                    }
                }
            }
            // TC_ENDBLOCKDATA = 0x78
            0x78 => {
                break;
            }
            _ => {
                i += 1;
            }
        }
    }

    if !found_class {
        result.push_str("(binary data)");
    }

    result
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Msgpack tests ---

    #[test]
    fn test_msgpack_positive_int() {
        // Msgpack positive fixint: 42 = 0x2a
        let bytes = [0x2a];
        let d = detect_and_decode(&bytes);
        assert_eq!(d.format, DetectedFormat::Msgpack);
        assert!(d.decoded.is_some());
    }

    #[test]
    fn test_msgpack_fixstr() {
        // Msgpack fixstr: "hi" = 0xa2 0x68 0x69
        let bytes = [0xa2, 0x68, 0x69];
        let d = detect_and_decode(&bytes);
        assert_eq!(d.format, DetectedFormat::Msgpack);
    }

    // --- Pickle tests ---

    #[test]
    fn test_pickle_short_binunicode() {
        // Pickle protocol 4: 0x80 (PROTO) 0x04 (version) 0x8c (SHORT_BINUNICODE) 0x05 "hello" 0x94 (MEMOIZE) 0x2e (STOP)
        let bytes = [
            0x80, 0x04, 0x8c, 0x05, b'h', b'e', b'l', b'l', b'o', 0x94, 0x2e,
        ];
        let d = detect_and_decode(&bytes);
        assert_eq!(d.format, DetectedFormat::Pickle);
        assert!(d.decoded.unwrap().contains("hello"));
    }

    // --- PHP Serialize tests ---

    #[test]
    fn test_php_serialize_string() {
        // s:5:"hello";
        let bytes = b"s:5:\"hello\";";
        let d = detect_and_decode(bytes);
        assert_eq!(d.format, DetectedFormat::PhpSerialize);
    }

    #[test]
    fn test_php_serialize_array() {
        // a:2:{i:0;s:3:"foo";i:1;s:3:"bar";}
        let bytes = b"a:2:{i:0;s:3:\"foo\";i:1;s:3:\"bar\";}";
        let d = detect_and_decode(bytes);
        assert_eq!(d.format, DetectedFormat::PhpSerialize);
    }

    // --- Java Serialize tests ---

    #[test]
    fn test_java_serialize() {
        // Magic 0xAC 0xED, version 0x00 0x05, then TC_CLASSDESC 0x72
        let mut bytes = vec![0xAC, 0xED, 0x00, 0x05, 0x72];
        // class name length (2 bytes) + name
        let name = "java.lang.String";
        bytes.extend_from_slice(&(name.len() as u16).to_be_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0x78); // TC_ENDBLOCKDATA
        let d = detect_and_decode(&bytes);
        assert_eq!(d.format, DetectedFormat::JavaSerialize);
        assert!(d.decoded.unwrap().contains("java.lang.String"));
    }

    // --- Compression tests ---

    #[test]
    fn test_gzip_json() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let json = r#"{"key":"value","num":42}"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        let d = detect_and_decode(&compressed);
        assert_eq!(d.compression, Some("gzip".into()));
        assert_eq!(d.format, DetectedFormat::Json);
        assert!(d.decoded.unwrap().contains("key"));
    }

    #[test]
    fn test_zstd_text() {
        let text = "hello world";
        let compressed = zstd::encode_all(text.as_bytes(), 3).unwrap();

        let d = detect_and_decode(&compressed);
        assert_eq!(d.compression, Some("zstd".into()));
        assert_eq!(d.format, DetectedFormat::Text);
    }

    // --- JSON tests ---

    #[test]
    fn test_json_object() {
        let bytes = r#"{"foo":"bar"}"#.as_bytes();
        let d = detect_and_decode(bytes);
        assert_eq!(d.format, DetectedFormat::Json);
        assert!(d.decoded.is_some());
    }

    #[test]
    fn test_json_array() {
        let bytes = b"[1,2,3]";
        let d = detect_and_decode(bytes);
        assert_eq!(d.format, DetectedFormat::Json);
    }

    // --- Text tests ---

    #[test]
    fn test_text() {
        let bytes = b"hello world";
        let d = detect_and_decode(bytes);
        assert_eq!(d.format, DetectedFormat::Text);
        assert!(d.decoded.is_none());
    }

    #[test]
    fn test_pickle_not_false_positive_on_normal_text() {
        // 回归：首字节为 'c'/'b'/'(' 的普通文本不应被误判为 Pickle
        for text in [b"compress me".as_slice(), b"hello world".as_slice()] {
            let d = detect_and_decode(text);
            assert_ne!(
                d.format,
                DetectedFormat::Pickle,
                "普通文本不应判为 Pickle: {:?}",
                text
            );
        }
    }

    #[test]
    fn test_pickle_not_false_positive_on_php_bool() {
        // 回归：PHP `b:1;` 以 'b' 开头，不应被 BINBYTES 误判为 Pickle
        let d = detect_and_decode(b"b:1;");
        assert_ne!(d.format, DetectedFormat::Pickle);
    }

    #[test]
    fn test_pickle_globals_must_have_structure() {
        // 回归：单独 0x63 后无 `module\nclass\n` 不应判为 Pickle
        let d = detect_and_decode(&[0x63, b'c', b'o', b'm', b'p', b'r', b'e', b's', b's']);
        assert_ne!(d.format, DetectedFormat::Pickle);
    }

    // --- Binary tests ---

    #[test]
    fn test_binary() {
        let bytes = [0x00, 0x01, 0x02, 0x03];
        let d = detect_and_decode(&bytes);
        assert_eq!(d.format, DetectedFormat::Binary);
    }

    // --- Edge cases ---

    #[test]
    fn test_empty() {
        let d = detect_and_decode(&[]);
        assert_eq!(d.format, DetectedFormat::Text);
        assert_eq!(d.raw_size, 0);
    }

    #[test]
    fn test_format_name() {
        assert_eq!(DetectedFormat::Msgpack.name(), "msgpack");
        assert_eq!(DetectedFormat::PhpSerialize.name(), "php_serialize");
        assert_eq!(DetectedFormat::JavaSerialize.name(), "java_serialize");
        assert_eq!(DetectedFormat::Pickle.name(), "pickle");
        assert_eq!(DetectedFormat::Text.name(), "text");
        assert_eq!(DetectedFormat::Binary.name(), "binary");
    }

    // --- Additional PHPSerialize / compression edge cases ---

    #[test]
    fn test_php_serialize_object() {
        // O:4:"User":1:{...}
        let bytes = b"O:4:\"User\":1:{s:3:\"age\";i:30;}";
        let d = detect_and_decode(bytes);
        assert_eq!(d.format, DetectedFormat::PhpSerialize);
    }

    #[test]
    fn test_php_serialize_bool_null_double() {
        assert_eq!(
            detect_and_decode(b"N;").format,
            DetectedFormat::PhpSerialize
        );
        assert_eq!(
            detect_and_decode(b"d:3.14;").format,
            DetectedFormat::PhpSerialize
        );
        // 普通文本不以 PHP 标记开头，不应误判
        assert_ne!(
            detect_and_decode(b"hello").format,
            DetectedFormat::PhpSerialize
        );
    }

    #[test]
    fn test_zlib_json() {
        // 使用 zlib 容器（0x78 头部），与 detect_compressed 的 zlib 分支匹配
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let json = r#"{"zlib":"ok","n":1}"#;
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(json.as_bytes()).unwrap();
        let compressed = enc.finish().unwrap();

        let d = detect_and_decode(&compressed);
        assert_eq!(d.compression, Some("zlib".into()));
        assert_eq!(d.format, DetectedFormat::Json);
        assert!(d.decoded.unwrap().contains("zlib"));
    }

    #[test]
    fn test_gzip_of_text() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let text = "plain text payload";
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(text.as_bytes()).unwrap();
        let compressed = enc.finish().unwrap();

        let d = detect_and_decode(&compressed);
        assert_eq!(d.compression, Some("gzip".into()));
        assert_eq!(d.format, DetectedFormat::Text);
    }
}
