use crate::db::Database;
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    Latency,    // 延迟 (毫秒)
    Throughput, // 吞吐量 (字节/秒)
    Error,      // 错误计数
    Connection, // 连接事件
}

impl MetricType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricType::Latency => "latency",
            MetricType::Throughput => "throughput",
            MetricType::Error => "error",
            MetricType::Connection => "connection",
        }
    }
}

impl From<&str> for MetricType {
    fn from(s: &str) -> Self {
        match s {
            "latency" => MetricType::Latency,
            "throughput" => MetricType::Throughput,
            "error" => MetricType::Error,
            "connection" => MetricType::Connection,
            _ => MetricType::Latency, // 默认
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub resource_id: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub tags: Option<String>, // JSON 扩展标签
    pub recorded_at: DateTime<Utc>,
}

impl Metric {
    pub fn new<S: Into<String>>(
        resource_id: S,
        metric_type: MetricType,
        value: f64,
        tags: Option<String>,
    ) -> Self {
        Self {
            resource_id: resource_id.into(),
            metric_type,
            value,
            tags,
            recorded_at: Utc::now(),
        }
    }
}

/// 指标采集器
pub struct MetricsCollector {
    db: Arc<Database>,
    cleanup_interval_secs: u64,
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
    shutdown_signal: Arc<RwLock<bool>>,
}

impl MetricsCollector {
    pub fn new(db: Arc<Database>, cleanup_interval_secs: u64) -> Self {
        Self {
            db,
            cleanup_interval_secs,
            cleanup_task: None,
            shutdown_signal: Arc::new(RwLock::new(false)),
        }
    }

    /// 获取系统健康状态
    pub async fn get_health(&self) -> Result<HealthStatus, anyhow::Error> {
        let conn = self.db.pool.get()?;

        // 获取数据库统计信息
        let mut db_stats = serde_json::Map::new();

        // 查询各表的行数
        let tables = vec!["environments", "resources", "audit_log", "metrics"];
        for table in tables {
            let count: i64 = conn.query_row(
                &format!("SELECT COUNT(*) FROM {}", table),
                [],
                |row| row.get(0),
            )?;
            db_stats.insert(table.to_string(), serde_json::Value::Number(count.into()));
        }

        // 获取数据库文件大小（对于SQLite）
        let db_size: i64 = conn.query_row(
            "SELECT page_count * page_size AS size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )?;

        db_stats.insert("size_bytes".to_string(), serde_json::Value::Number(db_size.into()));

        // TODO: 实际项目中应该添加系统指标（CPU、内存、磁盘等）
        // 这里先返回模拟数据
        let health_status = HealthStatus {
            status: "healthy".to_string(),
            uptime_seconds: 0, // TODO: 实际运行时间
            version: env!("CARGO_PKG_VERSION").to_string(),
            database: DbStats {
                size_bytes: db_size as u64,
                tables: TablesStats {
                    environments: db_stats.get("environments").unwrap().as_u64().unwrap(),
                    resources: db_stats.get("resources").unwrap().as_u64().unwrap(),
                    audit_log: db_stats.get("audit_log").unwrap().as_u64().unwrap(),
                    metrics: db_stats.get("metrics").unwrap().as_u64().unwrap(),
                }
            },
            system: SystemStats {
                cpu_usage_percent: 0.0, // TODO: 实际CPU使用率
                memory_usage_percent: 0.0, // TODO: 实际内存使用率
                disk_usage_percent: 0.0, // TODO: 实际磁盘使用率
            },
            connections: ConnectionStatsInfo {
                agents_online: 0, // TODO: 实际在线Agent数
                agents_total: 0, // TODO: 总Agent数
                active_sessions: 0, // TODO: 实际活跃会话数
            }
        };

        Ok(health_status)
    }

    /// 获取指标摘要
    pub async fn get_metrics_summary(&self, resource_id: Option<String>, hours: u32) -> Result<MetricsSummary, anyhow::Error> {
        self.get_summary(resource_id, hours).await
    }

    /// 获取时间序列数据
    pub async fn get_metrics_timeline(&self, resource_id: Option<String>, metric_type: MetricType, hours: u32, granularity: Option<String>) -> Result<Vec<TimePoint>, anyhow::Error> {
        self.get_timeline(resource_id, metric_type, hours, granularity).await
    }

    /// 记录一个指标
    pub async fn record_metric(&self, metric: Metric) -> Result<(), anyhow::Error> {
        let conn = self.db.pool.get()?;

        conn.execute(
            "INSERT INTO metrics (resource_id, metric_type, value, tags, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                metric.resource_id,
                metric.metric_type.as_str(),
                metric.value,
                metric.tags,
                metric.recorded_at.to_rfc3339()
            ],
        )?;

        debug!(
            "Recorded metric: {}={:.2} for resource {}",
            metric.metric_type.as_str(),
            metric.value,
            metric.resource_id
        );

        Ok(())
    }

    /// 记录延迟指标 (毫秒)
    pub async fn record_latency<S: Into<String>>(
        &self,
        resource_id: S,
        latency_ms: f64,
        tags: Option<String>,
    ) -> Result<(), anyhow::Error> {
        self.record_metric(Metric::new(
            resource_id,
            MetricType::Latency,
            latency_ms,
            tags,
        )).await
    }

    /// 记录吞吐量指标 (字节/秒)
    pub async fn record_throughput<S: Into<String>>(
        &self,
        resource_id: S,
        bytes_per_sec: f64,
        tags: Option<String>,
    ) -> Result<(), anyhow::Error> {
        self.record_metric(Metric::new(
            resource_id,
            MetricType::Throughput,
            bytes_per_sec,
            tags,
        )).await
    }

    /// 记录错误计数
    pub async fn record_error<S: Into<String>>(
        &self,
        resource_id: S,
        count: f64,
        tags: Option<String>,
    ) -> Result<(), anyhow::Error> {
        self.record_metric(Metric::new(
            resource_id,
            MetricType::Error,
            count,
            tags,
        )).await
    }

    /// 记录连接事件
    pub async fn record_connection<S: Into<String>>(
        &self,
        resource_id: S,
        connected: bool,
        tags: Option<String>,
    ) -> Result<(), anyhow::Error> {
        self.record_metric(Metric::new(
            resource_id,
            MetricType::Connection,
            if connected { 1.0 } else { 0.0 },
            tags,
        )).await
    }

    /// 获取指标摘要 (最近 N 小时)
    pub async fn get_summary(
        &self,
        resource_id: Option<String>,
        hours: u32,
    ) -> Result<MetricsSummary, anyhow::Error> {
        let conn = self.db.pool.get()?;
        let since = Utc::now() - chrono::Duration::hours(hours as i64);
        let since_str = since.to_rfc3339();

        let mut query = String::from(
            "SELECT metric_type, value FROM metrics WHERE recorded_at >= ?1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(since_str)];

        if let Some(ref rid) = resource_id {
            query.push_str(" AND resource_id = ?");
            params.push(Box::new(rid.as_str()));
        }

        let mut stmt = conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|boxed| boxed.as_ref()).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;

        let mut latency_values = Vec::new();
        let mut throughput_values = Vec::new();
        let mut error_count = 0.0;
        let mut connection_success = 0;
        let mut connection_total = 0;

        while let Some(row) = rows.next()? {
            let metric_type: String = row.get(0)?;
            let value: f64 = row.get(1)?;

            match metric_type.as_str() {
                "latency" => latency_values.push(value),
                "throughput" => throughput_values.push(value),
                "error" => error_count += value,
                "connection" => {
                    connection_total += 1;
                    if value > 0.0 {
                        connection_success += 1;
                    }
                }
                _ => {}
            }
        }

        // 计算延迟统计
        let mut latency_avg = 0.0;
        let mut latency_p50 = 0.0;
        let mut latency_p95 = 0.0;
        let mut latency_p99 = 0.0;
        let mut latency_min = 0.0;
        let mut latency_max = 0.0;

        if !latency_values.is_empty() {
            let mut sorted = latency_values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            latency_min = *sorted.first().unwrap();
            latency_max = *sorted.last().unwrap();
            latency_avg = latency_values.iter().sum::<f64>() / latency_values.len() as f64;
            latency_p50 = percentile_f64(&sorted, 0.5);
            latency_p95 = percentile_f64(&sorted, 0.95);
            latency_p99 = percentile_f64(&sorted, 0.99);
        }

        // 计算吞吐量统计
        let mut throughput_avg = 0.0;
        let mut throughput_total = 0.0;

        if !throughput_values.is_empty() {
            throughput_avg = throughput_values.iter().sum::<f64>() / throughput_values.len() as f64;
            throughput_total = throughput_values.iter().sum::<f64>();
        }

        let error_rate = if connection_total > 0 {
            (error_count / connection_total as f64) * 100.0
        } else {
            0.0
        };

        Ok(MetricsSummary {
            latency: LatencyStats {
                avg_ms: latency_avg,
                p50_ms: latency_p50,
                p95_ms: latency_p95,
                p99_ms: latency_p99,
                min_ms: latency_min,
                max_ms: latency_max,
            },
            throughput: ThroughputStats {
                avg_bytes_per_sec: throughput_avg,
                total_bytes: throughput_total,
            },
            errors: ErrorStats {
                total: error_count as u64,
                error_rate_percent: error_rate,
            },
            connections: ConnectionStats {
                total: connection_total as u64,
                successful: connection_success as u64,
                failed: (connection_total - connection_success) as u64,
            },
            recorded_at: Utc::now(),
        })
    }

    /// 获取时间序列数据 (按时间窗口分桶)
    pub async fn get_timeline(
        &self,
        resource_id: Option<String>,
        metric_type: MetricType,
        hours: u32,
        granularity: Option<String>,
    ) -> Result<Vec<TimePoint>, anyhow::Error> {
        let conn = self.db.pool.get()?;
        let since = Utc::now() - chrono::Duration::hours(hours as i64);
        let since_str = since.to_rfc3339();

        let mut query = String::from(
            "SELECT metric_type, value, recorded_at FROM metrics WHERE recorded_at >= ?1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(since_str)];

        if let Some(ref rid) = resource_id {
            query.push_str(" AND resource_id = ?");
            params.push(Box::new(rid.as_str()));
        }

        let mt_str = metric_type.as_str();
        query.push_str(" AND metric_type = ?");
        params.push(Box::new(mt_str));

        // 添加排序
        query.push_str(" ORDER BY recorded_at");

        let mut stmt = conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|boxed| boxed.as_ref()).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;

        // 根据粒度分组
        let mut buckets: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();

        while let Some(row) = rows.next()? {
            let _metric_type: String = row.get(0)?;
            let value: f64 = row.get(1)?;
            let timestamp: String = row.get(2)?;

            // 简单的时间桶实现（按小时取整）
            let bucket_key = if let Some(gran) = &granularity {
                match gran.as_str() {
                    "1h" => {
                        // 按小时分组
                        let dt = DateTime::parse_from_rfc3339(&timestamp).unwrap();
                        dt.format("%Y-%m-%d %H:00:00").to_string()
                    }
                    "6h" => {
                        // 按6小时分组
                        let dt = DateTime::parse_from_rfc3339(&timestamp).unwrap();
                        let hour = dt.hour();
                        let bucket = (hour / 6) * 6;
                        dt.format(&format!("{}-{:02}:00:00", dt.format("%Y-%m-%d"), bucket)).to_string()
                    }
                    "24h" => {
                        // 按天分组
                        let dt = DateTime::parse_from_rfc3339(&timestamp).unwrap();
                        dt.format("%Y-%m-%d 00:00:00").to_string()
                    }
                    "7d" => {
                        // 按周分组
                        let dt = DateTime::parse_from_rfc3339(&timestamp).unwrap();
                        dt.format("%Y-%W-1 00:00:00").to_string()
                    }
                    _ => {
                        // 默认按小时分组
                        let dt = DateTime::parse_from_rfc3339(&timestamp).unwrap();
                        dt.format("%Y-%m-%d %H:00:00").to_string()
                    }
                }
            } else {
                // 自动根据时间范围选择粒度
                let dt = DateTime::parse_from_rfc3339(&timestamp).unwrap();
                if hours <= 24 {
                    dt.format("%Y-%m-%d %H:00:00").to_string()
                } else if hours <= 168 { // 一周
                    dt.format("%Y-%m-%d 00:00:00").to_string()
                } else {
                    dt.format("%Y-%W-1 00:00:00").to_string()
                }
            };

            buckets.entry(bucket_key).or_insert_with(Vec::new).push(value);
        }

        // 计算每个桶的平均值和数量
        let mut result = Vec::new();
        for (bucket_key, values) in buckets {
            if !values.is_empty() {
                let sum: f64 = values.iter().sum();
                let avg = sum / values.len() as f64;
                let count = values.len() as u32;
                result.push(TimePoint {
                    timestamp: format!("{}Z", bucket_key),
                    value: avg,
                    count,
                });
            }
        }

        // 按时间戳排序
        result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(result)
    }

    /// 清理超过保留期限的旧数据
    pub async fn cleanup_old_data(&self, retention_days: u64) -> Result<usize, anyhow::Error> {
        let conn = self.db.pool.get()?;
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = conn.execute(
            "DELETE FROM metrics WHERE recorded_at < ?1",
            rusqlite::params![cutoff_str],
        )?;

        info!("Cleaned up {} old metric records (older than {} days)", deleted, retention_days);
        Ok(deleted as usize)
    }

    /// 启动后台清理任务
    pub fn start_cleanup_task(&mut self) {
        let db = self.db.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        let interval_secs = self.cleanup_interval_secs;

        self.cleanup_task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;

                // 检查是否应该关闭
                let should_shutdown = *shutdown_signal.read().await;
                if should_shutdown {
                    break;
                }

                // 执行清理（保留7天数据）
                if let Ok(db) = db.pool.get() {
                    let cutoff = Utc::now() - chrono::Duration::days(7);
                    let cutoff_str = cutoff.to_rfc3339();
                    let _ = db.execute(
                        "DELETE FROM metrics WHERE recorded_at < ?1",
                        rusqlite::params![cutoff_str],
                    );
                }
            }
        }));
    }

    /// 停止后台清理任务
    pub async fn stop_cleanup_task(&self) {
        *self.shutdown_signal.write().await = true;
        if let Some(task) = &self.cleanup_task {
            let _ = task;
        }
    }
}



/// 计算百分位数的辅助函数
fn percentile_f64(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let index = ((sorted.len() as f64 - 1.0) * percentile) as usize;
    let fraction = ((sorted.len() as f64 - 1.0) * percentile) - index as f64;

    if index + 1 >= sorted.len() {
        return sorted[sorted.len() - 1];
    }

    let low = sorted[index];
    let high = sorted[index + 1];
    low + (high - low) * fraction
}


/// 指标摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub latency: LatencyStats,
    pub throughput: ThroughputStats,
    pub errors: ErrorStats,
    pub connections: ConnectionStats,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputStats {
    pub avg_bytes_per_sec: f64,
    pub total_bytes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    pub total: u64,
    pub error_rate_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
}

/// 时间序列数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePoint {
    pub timestamp: String,
    pub value: f64,
    pub count: u32,
}

/// 系统健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String, // healthy | degraded | unhealthy
    pub uptime_seconds: u64,
    pub version: String,
    pub database: DbStats,
    pub system: SystemStats,
    pub connections: ConnectionStatsInfo,
}

/// 数据库统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub size_bytes: u64,
    pub tables: TablesStats,
}

/// 数据库表统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablesStats {
    pub environments: u64,
    pub resources: u64,
    pub audit_log: u64,
    pub metrics: u64,
}

/// 系统资源统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
}

/// 连接统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatsInfo {
    pub agents_online: u64,
    pub agents_total: u64,
    pub active_sessions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_record_and_get_metrics() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let collector = MetricsCollector::new(db.clone(), 3600);

        // 插入环境记录（外键约束所必需的）
        let resource_time = "2024-01-01T00:00:00Z";
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                "env_001",
                "test-environment",
                "test environment for metrics",
                "ssh",
                resource_time,
                resource_time
            ],
        );

        // 插入资源记录（外键约束所必需的）
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                "resource_1",
                "env_001",
                "test-resource-1",
                "ssh",
                None::<String>,
                "{}",
                "ready",
                resource_time,
                resource_time
            ],
        );

        // 记录一些测试数据
        let _ = collector.record_latency("resource_1", 10.0, None).await;
        let _ = collector.record_latency("resource_1", 20.0, None).await;
        let _ = collector.record_latency("resource_1", 30.0, None).await;
        let _ = collector.record_throughput("resource_1", 1000.0, None).await;
        let _ = collector.record_error("resource_1", 1.0, None).await;
        let _ = collector.record_connection("resource_1", true, None).await;

        // 获取摘要
        let summary = collector.get_summary(Some("resource_1".to_string()), 24).await.unwrap();

        assert_eq!(summary.latency.avg_ms, 20.0);
        assert_eq!(summary.latency.min_ms, 10.0);
        assert_eq!(summary.latency.max_ms, 30.0);
        assert_eq!(summary.throughput.avg_bytes_per_sec, 1000.0);
        assert_eq!(summary.errors.total, 1);
        assert_eq!(summary.connections.total, 1);
        assert_eq!(summary.connections.successful, 1);
    }

    #[tokio::test]
    async fn test_cleanup_old_data() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let collector = MetricsCollector::new(db.clone(), 3600);

        // 插入环境记录（外键约束所必需的）
        let resource_time = "2024-01-01T00:00:00Z";
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO environments (id, name, description, connection_mode, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                "env_001",
                "test-environment",
                "test environment for metrics",
                "ssh",
                resource_time,
                resource_time
            ],
        );

        // 插入资源记录（外键约束所必需的）
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                "old_resource",
                "env_001",
                "old-test-resource",
                "ssh",
                None::<String>,
                "{}",
                "ready",
                resource_time,
                resource_time
            ],
        );
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                "new_resource",
                "env_001",
                "new-test-resource",
                "ssh",
                None::<String>,
                "{}",
                "ready",
                resource_time,
                resource_time
            ],
        );
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                "old_resource",
                "env_001",
                "old-test-resource",
                "ssh",
                None::<String>,
                "{}",
                "ready",
                resource_time,
                resource_time
            ],
        );
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO resources (id, environment_id, name, protocol, agent_id, config_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                "new_resource",
                "env_001",
                "new-test-resource",
                "ssh",
                None::<String>,
                "{}",
                "ready",
                resource_time,
                resource_time
            ],
        );

        // 插入一条旧数据（8天前）
        let old_time = Utc::now() - chrono::Duration::days(8);
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO metrics (resource_id, metric_type, value, tags, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "old_resource",
                "latency",
                100.0,
                None::<String>,
                old_time.to_rfc3339()
            ],
        );

        // 插入一条新数据（1天前）
        let new_time = Utc::now() - chrono::Duration::days(1);
        let _ = db.pool.get().unwrap().execute(
            "INSERT INTO metrics (resource_id, metric_type, value, tags, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "new_resource",
                "latency",
                50.0,
                None::<String>,
                new_time.to_rfc3339()
            ],
        );

        // 清理7天以上的数据
        let deleted = collector.cleanup_old_data(7).await.unwrap();
        assert_eq!(deleted, 1); // 应该只删除了1条旧数据

        // 验证新数据仍然存在
        let count = db.pool.get().unwrap().query_row(
            "SELECT COUNT(*) FROM metrics",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap();
        assert_eq!(count, 1);
    }
}