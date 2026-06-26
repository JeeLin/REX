use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

/// 一条日志条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// 初始化 tracing，同时输出到 stdout 和 LogCollector
pub fn init_tracing_with_collector(collector: LogCollector) {
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(env_filter);

    let collector_layer = LogCollectorLayer::new(collector);

    let subscriber = tracing_subscriber::registry()
        .with(fmt_layer)
        .with(collector_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set global tracing subscriber");
}

/// Agent 日志收集器，环形缓冲区存储最近 1000 条日志
#[derive(Clone)]
pub struct LogCollector {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    since: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl LogCollector {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            since: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// 添加一条日志
    pub fn add(&self, entry: LogEntry) {
        let mut entries = self.entries.lock().unwrap();
        let mut since = self.since.lock().unwrap();

        entries.push_back(entry.clone());
        since.push_back(entry);

        // 淘汰超过 1 小时的条目
        self.expire_old(&mut entries, &mut since);
    }

    /// drain 自上次 drain 以来新增的日志（用于心跳上报）
    pub fn drain_since(&self) -> Vec<LogEntry> {
        let mut since = self.since.lock().unwrap();
        since.drain(..).collect()
    }

    /// 获取所有日志（用于查询 API）
    pub fn get_all(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap().iter().cloned().collect()
    }

    /// 获取指定时间之后的日志
    pub fn get_since(&self, since_ts: &str) -> Vec<LogEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.timestamp.as_str() > since_ts)
            .cloned()
            .collect()
    }

    /// 当前日志数量
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    fn expire_old(
        &self,
        entries: &mut VecDeque<LogEntry>,
        since: &mut VecDeque<LogEntry>,
    ) {
        // 保留最近 1000 条
        while entries.len() > 1000 {
            entries.pop_front();
        }
        while since.len() > 1000 {
            since.pop_front();
        }
    }
}

/// 自定义 tracing Layer，将日志转发到 LogCollector
pub struct LogCollectorLayer {
    collector: LogCollector,
}

impl LogCollectorLayer {
    pub fn new(collector: LogCollector) -> Self {
        Self { collector }
    }
}

impl<S> Layer<S> for LogCollectorLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let meta = event.metadata();
        let level = match *meta.level() {
            Level::ERROR => "error",
            Level::WARN => "warn",
            Level::INFO => "info",
            Level::DEBUG => "debug",
            Level::TRACE => "trace",
        };

        // 提取消息
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        if visitor.0.is_empty() {
            return;
        }

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        self.collector.add(LogEntry {
            timestamp: now,
            level: level.to_string(),
            message: visitor.0,
        });
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
            // 去掉引号
            if self.0.starts_with('"') && self.0.ends_with('"') {
                self.0 = self.0[1..self.0.len() - 1].to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get_all() {
        let collector = LogCollector::new();
        collector.add(LogEntry {
            timestamp: "2026-06-26T10:00:00Z".to_string(),
            level: "info".to_string(),
            message: "test message".to_string(),
        });
        assert_eq!(collector.len(), 1);
        let all = collector.get_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].message, "test message");
    }

    #[test]
    fn drain_since_clears_buffer() {
        let collector = LogCollector::new();
        collector.add(LogEntry {
            timestamp: "2026-06-26T10:00:00Z".to_string(),
            level: "info".to_string(),
            message: "msg1".to_string(),
        });
        collector.add(LogEntry {
            timestamp: "2026-06-26T10:01:00Z".to_string(),
            level: "warn".to_string(),
            message: "msg2".to_string(),
        });

        let drained = collector.drain_since();
        assert_eq!(drained.len(), 2);

        // 第二次 drain 应为空
        let drained2 = collector.drain_since();
        assert_eq!(drained2.len(), 0);

        // 原始数据仍在
        assert_eq!(collector.len(), 2);
    }

    #[test]
    fn get_since_filters_correctly() {
        let collector = LogCollector::new();
        collector.add(LogEntry {
            timestamp: "2026-06-26T10:00:00Z".to_string(),
            level: "info".to_string(),
            message: "old".to_string(),
        });
        collector.add(LogEntry {
            timestamp: "2026-06-26T10:05:00Z".to_string(),
            level: "info".to_string(),
            message: "new".to_string(),
        });

        let filtered = collector.get_since("2026-06-26T10:02:00Z");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message, "new");
    }

    #[test]
    fn capacity_limit() {
        let collector = LogCollector::new();
        for i in 0..1100 {
            collector.add(LogEntry {
                timestamp: format!("2026-06-26T10:{:02}:00Z", i % 60),
                level: "info".to_string(),
                message: format!("msg{}", i),
            });
        }
        // 应保留最多 1000 条
        assert!(collector.len() <= 1000);
    }
}
