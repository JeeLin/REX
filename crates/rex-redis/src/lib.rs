//! Redis 协议实现 — 基于 redis crate 的 RedisConnector。

use anyhow::{Context, Result};
use redis::cmd as redis_cmd;
use rex_common::redis::{
    DbInfo, FormatInfo, KeyInfo, KeyspaceInfo, RedisConnectRequest, RedisConnector, RedisInfo,
    RedisValue,
};

pub mod redis_codec;

/// Redis 连接器
pub struct RedisConnectorImpl {
    conn: redis::aio::ConnectionManager,
}

impl RedisConnectorImpl {
    /// 建立 Redis 连接
    pub async fn connect(req: RedisConnectRequest) -> Result<Self> {
        let url = if let Some(ref password) = req.password {
            format!("redis://:{}@{}:{}", password, req.host, req.port)
        } else {
            format!("redis://{}:{}", req.host, req.port)
        };

        let client = redis::Client::open(url.as_str())
            .with_context(|| format!("failed to create Redis client for {url}"))?;

        let mut conn = client
            .get_connection_manager()
            .await
            .with_context(|| format!("failed to connect to Redis at {}:{}", req.host, req.port))?;

        // 选择 DB
        if let Some(db) = req.db {
            redis_cmd("SELECT")
                .arg(db)
                .query_async::<()>(&mut conn)
                .await
                .with_context(|| format!("failed to select DB {db}"))?;
        }

        Ok(Self { conn })
    }

    fn parse_keyspace(info_str: &str) -> Vec<KeyspaceInfo> {
        let mut result = Vec::new();
        for line in info_str.lines() {
            if let Some(rest) = line.strip_prefix("db") {
                if let Some((db_num, stats)) = rest.split_once(':') {
                    let mut keys = 0u64;
                    let mut expires = 0u64;
                    for part in stats.split(',') {
                        if let Some(v) = part.strip_prefix("keys=") {
                            keys = v.parse().unwrap_or(0);
                        } else if let Some(v) = part.strip_prefix("expires=") {
                            expires = v.parse().unwrap_or(0);
                        }
                    }
                    result.push(KeyspaceInfo {
                        db: format!("db{db_num}"),
                        keys,
                        expires,
                    });
                }
            }
        }
        result
    }
}

#[async_trait::async_trait]
impl RedisConnector for RedisConnectorImpl {
    async fn info(&mut self) -> Result<RedisInfo> {
        let info_str: String = redis_cmd("INFO")
            .arg("server")
            .arg("memory")
            .arg("clients")
            .arg("stats")
            .arg("keyspace")
            .query_async(&mut self.conn)
            .await
            .context("failed to execute INFO")?;

        let mut redis_version = String::new();
        let mut os = String::new();
        let mut process_id = String::new();
        let mut connected_clients = String::new();
        let mut used_memory = String::new();
        let mut used_memory_peak = String::new();
        let mut total_commands_processed = String::new();
        let mut keyspace_section = String::new();
        let mut in_keyspace = false;

        for line in info_str.lines() {
            if line.starts_with('#') {
                in_keyspace = line.contains("keyspace");
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let v = v.trim();
                match k {
                    "redis_version" => redis_version = v.to_string(),
                    "os" => os = v.to_string(),
                    "process_id" => process_id = v.to_string(),
                    "connected_clients" => connected_clients = v.to_string(),
                    "used_memory" => used_memory = v.to_string(),
                    "used_memory_peak" => used_memory_peak = v.to_string(),
                    "total_commands_processed" => total_commands_processed = v.to_string(),
                    _ => {}
                }
                if in_keyspace {
                    keyspace_section.push_str(line);
                    keyspace_section.push('\n');
                }
            }
        }

        Ok(RedisInfo {
            redis_version,
            os,
            process_id,
            connected_clients,
            used_memory,
            used_memory_peak,
            total_commands_processed,
            keyspace: Self::parse_keyspace(&keyspace_section),
        })
    }

    async fn dbs(&mut self) -> Result<Vec<DbInfo>> {
        let info_str: String = redis_cmd("INFO")
            .arg("keyspace")
            .query_async(&mut self.conn)
            .await
            .context("failed to execute INFO keyspace")?;

        let keyspace = Self::parse_keyspace(&info_str);
        Ok(keyspace
            .into_iter()
            .map(|ks| {
                let idx: i32 = ks.db.strip_prefix("db").unwrap_or("0").parse().unwrap_or(0);
                DbInfo {
                    index: idx,
                    keys: ks.keys,
                    expires: ks.expires,
                }
            })
            .collect())
    }

    async fn select_db(&mut self, db: i32) -> Result<()> {
        redis_cmd("SELECT")
            .arg(db)
            .query_async::<()>(&mut self.conn)
            .await
            .with_context(|| format!("failed to select DB {db}"))?;
        Ok(())
    }

    async fn scan(&mut self, pattern: &str, count: u32) -> Result<Vec<KeyInfo>> {
        // SCAN 返回 (cursor, keys)
        let (_cursor, raw_keys): (u64, Vec<String>) = redis_cmd("SCAN")
            .arg(0u64)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(count)
            .query_async(&mut self.conn)
            .await
            .context("failed to execute SCAN")?;

        // 逐个获取类型
        let mut result = Vec::with_capacity(raw_keys.len());
        for key in raw_keys {
            let type_name: String = redis_cmd("TYPE")
                .arg(&key)
                .query_async(&mut self.conn)
                .await
                .unwrap_or_default();
            result.push(KeyInfo { key, type_name });
        }
        Ok(result)
    }

    async fn get_type(&mut self, key: &str) -> Result<String> {
        let t: String = redis_cmd("TYPE")
            .arg(key)
            .query_async(&mut self.conn)
            .await
            .context("failed to execute TYPE")?;
        Ok(t)
    }

    async fn get_value(&mut self, key: &str) -> Result<RedisValue> {
        let type_name = self.get_type(key).await?;
        match type_name.as_str() {
            "string" => {
                let val: String = redis_cmd("GET")
                    .arg(key)
                    .query_async(&mut self.conn)
                    .await
                    .context("failed to execute GET")?;
                let detection = redis_codec::detect_and_decode(val.as_bytes());
                let format = Some(FormatInfo {
                    detected: detection.format.name().to_string(),
                    decoded: detection.decoded,
                    compression: detection.compression,
                });
                Ok(RedisValue::String { value: val, format })
            }
            "list" => {
                let val: Vec<String> = redis_cmd("LRANGE")
                    .arg(key)
                    .arg(0)
                    .arg(-1)
                    .query_async(&mut self.conn)
                    .await
                    .context("failed to execute LRANGE")?;
                Ok(RedisValue::List(val))
            }
            "set" => {
                let val: Vec<String> = redis_cmd("SMEMBERS")
                    .arg(key)
                    .query_async(&mut self.conn)
                    .await
                    .context("failed to execute SMEMBERS")?;
                Ok(RedisValue::Set(val))
            }
            "zset" => {
                let val: Vec<(String, f64)> = redis_cmd("ZRANGE")
                    .arg(key)
                    .arg(0)
                    .arg(-1)
                    .arg("WITHSCORES")
                    .query_async(&mut self.conn)
                    .await
                    .context("failed to execute ZRANGE")?;
                Ok(RedisValue::ZSet(val))
            }
            "hash" => {
                let val: Vec<(String, String)> = redis_cmd("HGETALL")
                    .arg(key)
                    .query_async(&mut self.conn)
                    .await
                    .context("failed to execute HGETALL")?;
                Ok(RedisValue::Hash(val))
            }
            _ => Ok(RedisValue::String {
                value: format!("[{type_name}]"),
                format: None,
            }),
        }
    }

    async fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        redis_cmd("SET")
            .arg(key)
            .arg(value)
            .query_async::<()>(&mut self.conn)
            .await
            .context("failed to execute SET")?;
        Ok(())
    }

    async fn del(&mut self, keys: &[String]) -> Result<u64> {
        if keys.is_empty() {
            return Ok(0);
        }
        let count: u64 = redis_cmd("DEL")
            .arg(keys)
            .query_async(&mut self.conn)
            .await
            .context("failed to execute DEL")?;
        Ok(count)
    }

    async fn ttl(&mut self, key: &str) -> Result<i64> {
        let ttl: i64 = redis_cmd("TTL")
            .arg(key)
            .query_async(&mut self.conn)
            .await
            .context("failed to execute TTL")?;
        Ok(ttl)
    }

    async fn set_ttl(&mut self, key: &str, seconds: i64) -> Result<()> {
        if seconds == -1 {
            redis_cmd("PERSIST")
                .arg(key)
                .query_async::<()>(&mut self.conn)
                .await
                .context("failed to execute PERSIST")?;
        } else if seconds == -2 {
            redis_cmd("DEL")
                .arg(key)
                .query_async::<()>(&mut self.conn)
                .await
                .context("failed to execute DEL")?;
        } else {
            redis_cmd("EXPIRE")
                .arg(key)
                .arg(seconds)
                .query_async::<()>(&mut self.conn)
                .await
                .context("failed to execute EXPIRE")?;
        }
        Ok(())
    }

    async fn command(&mut self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            anyhow::bail!("empty command");
        }
        let mut cmd = redis_cmd(&args[0]);
        for arg in &args[1..] {
            cmd.arg(arg);
        }
        let result: redis::RedisResult<redis::Value> = cmd.query_async(&mut self.conn).await;

        match result {
            Ok(val) => Ok(format!("{:?}", val)),
            Err(e) => Ok(format!("(error) {e}")),
        }
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
