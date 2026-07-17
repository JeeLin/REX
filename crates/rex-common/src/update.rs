//! 更新状态模型 — Hub 和 Agent 共享的版本检查、更新指令、进度跟踪。

use serde::{Deserialize, Serialize};

/// Hub 版本信息
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub hub_version: String,
    pub latest_version: Option<String>,
    pub download_url: Option<String>,
    pub agents: Vec<AgentVersionInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentVersionInfo {
    pub agent_id: String,
    pub name: String,
    pub version: String,
    pub is_online: bool,
    pub is_up_to_date: bool,
}

/// 更新检查结果
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: String,
    pub release_notes: String,
}

/// 更新阶段
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePhase {
    Idle,
    Downloading,
    Verifying,
    Replacing,
    Restarting,
    Error,
}

impl Default for UpdatePhase {
    fn default() -> Self {
        Self::Idle
    }
}

/// Agent 更新状态（Hub 侧内存中维护）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub phase: UpdatePhase,
    pub progress: f64,
    pub current_version: String,
    pub target_version: String,
    pub error: Option<String>,
    pub started_at: Option<String>,
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            phase: UpdatePhase::Idle,
            progress: 0.0,
            current_version: String::new(),
            target_version: String::new(),
            error: None,
            started_at: None,
        }
    }
}

/// 更新指令（Hub → Agent）
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCommand {
    pub version: String,
    pub download_url: String,
    pub fallback_url: String,
    pub sha256: String,
}

/// 更新进度（Agent → Hub）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProgress {
    pub phase: UpdatePhase,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub error: Option<String>,
}
