//! Hub ↔ Agent 协议会话消息契约（v0.70.6 子任务 #2）。
//!
//! 此前 agent 模式各协议的处理方式不统一：ssh 是「裸 TCP 管道 + hub 裸桥接」，
//! sql/redis/file 则是 hub 直接连目标、根本未走隧道。本模块定义一套结构化
//! 「协议会话」消息，使 Agent 在私网内终结协议（ssh/sql/redis/s3/sftp），只把
//! 已协商好的结果（终端流 / 查询结果 / 文件分块）经隧道回传 Hub。
//!
//! 帧约定（与 `crates/rex-agent/src/agent_ws.rs`、`crates/rex-hub/src/agent_ws.rs`
//! 一致）：控制/结构化消息用 `Text`(JSON) 帧；`[4B channelId][data]` 二进制帧
//! 仅用于大体积流式数据（终端输出、文件分块）。`request_id` 关联前端请求，
//! `channel_id` 关联 agent 隧道槽。

use serde::{Deserialize, Serialize};

/// Hub → Agent：发起一次协议会话。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOpen {
    /// 前端/Hub 侧请求标识，用于响应匹配。
    pub request_id: String,
    /// Agent 隧道槽标识（数值，对应二进制帧前缀）。
    pub channel_id: String,
    /// 协议类型：ssh | sql | redis | s3 | sftp。
    pub protocol: String,
    /// 协议专用配置（host/port/认证/库名/桶名等）。凭据经加密 WS 下发，由 Agent 使用。
    pub config: serde_json::Value,
}

/// Hub → Agent：对既有会话下发查询/命令/控制。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRequest {
    pub channel_id: String,
    /// 子请求类型：query | exec | list | metadata | read | write | resize | close。
    pub kind: String,
    /// 子请求序号，用于响应匹配（同一会话内多个查询并发）。
    pub seq: u64,
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Agent → Hub：会话连接成功。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOpened {
    pub request_id: String,
    pub channel_id: String,
    /// SQL 会话在 agent 侧探测出的子类（dialect，mysql/postgresql/sqlite）。
    /// Hub 据此回写资源 subtype 以缓存探测结果；非 SQL 会话或无需探测时为 None。
    #[serde(default)]
    pub subtype: Option<String>,
}

/// Agent → Hub：会话连接失败。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionError {
    pub channel_id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    pub error: String,
}

/// Agent → Hub：协议专用结果（查询响应 / 命令回显 / 元数据）。
///
/// `data` 为协议特定的 JSON 负载：SQL 用 `{columns, rows}`；Redis 用 `{value}`；
/// S3/SFTP 用 `{entries}` 等。二进制大体积走 `FileChunk` 而非此字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub channel_id: String,
    pub seq: u64,
    #[serde(default)]
    pub data: serde_json::Value,
    /// 可选错误信息（子请求级失败）。
    #[serde(default)]
    pub error: Option<String>,
}

/// Agent → Hub：文件分块（S3/SFTP 读写）。大体积数据走二进制帧结构，此处为元数据描述。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub channel_id: String,
    pub seq: u64,
    pub path: String,
    /// 分块在文件中的偏移（字节）。
    pub offset: u64,
    /// 是否分块序列的结尾。
    #[serde(default)]
    pub eof: bool,
    /// 当 `kind=meta` 时携带文件大小/是否目录等元信息；`kind=data` 时 `len` 为分块字节数。
    pub kind: String,
    pub len: u64,
}

/// 文本消息顶层信封：以 `type` 区分上述各消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentSessionMsg {
    #[serde(rename = "session_open")]
    SessionOpen(SessionOpen),
    #[serde(rename = "session_request")]
    SessionRequest(SessionRequest),
    #[serde(rename = "session_opened")]
    SessionOpened(SessionOpened),
    #[serde(rename = "session_error")]
    SessionError(SessionError),
    #[serde(rename = "session_response")]
    SessionResponse(SessionResponse),
    #[serde(rename = "file_chunk")]
    FileChunk(FileChunk),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_open_round_trip() {
        let msg = AgentSessionMsg::SessionOpen(SessionOpen {
            request_id: "req_1".into(),
            channel_id: "7".into(),
            protocol: "sql".into(),
            config: serde_json::json!({"host":"db","port":3306}),
        });
        let s = serde_json::to_string(&msg).unwrap();
        assert!(s.contains("session_open"));
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::SessionOpen(o) => {
                assert_eq!(o.protocol, "sql");
                assert_eq!(o.channel_id, "7");
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn session_response_round_trip() {
        let msg = AgentSessionMsg::SessionResponse(SessionResponse {
            channel_id: "7".into(),
            seq: 3,
            data: serde_json::json!({"columns":["id"],"rows":[[1]]}),
            error: None,
        });
        let s = serde_json::to_string(&msg).unwrap();
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::SessionResponse(r) => {
                assert_eq!(r.seq, 3);
                assert_eq!(r.data["rows"][0][0], 1);
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn file_chunk_round_trip() {
        let msg = AgentSessionMsg::FileChunk(FileChunk {
            channel_id: "9".into(),
            seq: 1,
            path: "/tmp/a".into(),
            offset: 0,
            eof: false,
            kind: "data".into(),
            len: 8192,
        });
        let s = serde_json::to_string(&msg).unwrap();
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::FileChunk(c) => {
                assert_eq!(c.len, 8192);
                assert!(!c.eof);
            }
            _ => panic!("unexpected"),
        }
    }
    #[test]
    fn session_request_round_trip() {
        let msg = AgentSessionMsg::SessionRequest(SessionRequest {
            channel_id: "3".into(),
            kind: "query".into(),
            seq: 1,
            payload: serde_json::json!({"sql": "SELECT 1"}),
        });
        let s = serde_json::to_string(&msg).unwrap();
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::SessionRequest(r) => {
                assert_eq!(r.kind, "query");
                assert_eq!(r.seq, 1);
                assert_eq!(r.payload["sql"], "SELECT 1");
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn session_opened_round_trip() {
        let msg = AgentSessionMsg::SessionOpened(SessionOpened {
            request_id: "req_2".into(),
            channel_id: "5".into(),
            subtype: Some("mysql".into()),
        });
        let s = serde_json::to_string(&msg).unwrap();
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::SessionOpened(o) => {
                assert_eq!(o.subtype.unwrap(), "mysql");
                assert_eq!(o.channel_id, "5");
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn session_error_round_trip() {
        let msg = AgentSessionMsg::SessionError(SessionError {
            channel_id: "5".into(),
            request_id: Some("req_2".into()),
            error: "connection refused".into(),
        });
        let s = serde_json::to_string(&msg).unwrap();
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::SessionError(e) => {
                assert_eq!(e.error, "connection refused");
                assert_eq!(e.request_id.unwrap(), "req_2");
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn session_error_without_request_id() {
        let msg = AgentSessionMsg::SessionError(SessionError {
            channel_id: "1".into(),
            request_id: None,
            error: "timeout".into(),
        });
        let s = serde_json::to_string(&msg).unwrap();
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::SessionError(e) => {
                assert!(e.request_id.is_none());
                assert_eq!(e.error, "timeout");
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn session_response_with_error() {
        let msg = AgentSessionMsg::SessionResponse(SessionResponse {
            channel_id: "4".into(),
            seq: 2,
            data: serde_json::json!({}),
            error: Some("table not found".into()),
        });
        let s = serde_json::to_string(&msg).unwrap();
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::SessionResponse(r) => {
                assert_eq!(r.error.unwrap(), "table not found");
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn session_opened_without_subtype() {
        let msg = AgentSessionMsg::SessionOpened(SessionOpened {
            request_id: "req_3".into(),
            channel_id: "10".into(),
            subtype: None,
        });
        let s = serde_json::to_string(&msg).unwrap();
        // serde_json serializes None as null (not skipped unless #[serde(skip_serializing_if)])
        assert!(s.contains("subtype"));
        assert!(s.contains("null"));
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::SessionOpened(o) => {
                assert!(o.subtype.is_none());
            }
            _ => panic!("unexpected"),
        }
    }

    #[test]
    fn file_chunk_eof_round_trip() {
        let msg = AgentSessionMsg::FileChunk(FileChunk {
            channel_id: "9".into(),
            seq: 5,
            path: "/data/export.csv".into(),
            offset: 1024,
            eof: true,
            kind: "data".into(),
            len: 512,
        });
        let s = serde_json::to_string(&msg).unwrap();
        let back: AgentSessionMsg = serde_json::from_str(&s).unwrap();
        match back {
            AgentSessionMsg::FileChunk(c) => {
                assert!(c.eof);
                assert_eq!(c.offset, 1024);
                assert_eq!(c.len, 512);
            }
            _ => panic!("unexpected"),
        }
    }
}
