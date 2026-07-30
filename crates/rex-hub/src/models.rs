use serde::{Deserialize, Serialize};

// --- Environment ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub connection_mode: String,
    pub registration_token: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewEnvironment {
    pub name: String,
    pub description: Option<String>,
    pub connection_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateEnvironment {
    pub name: Option<String>,
    pub description: Option<String>,
    pub connection_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDetail {
    #[serde(flatten)]
    pub environment: Environment,
    pub resource_count: i64,
    pub agent_status: Option<String>,
}

// --- Resource ---

#[derive(Debug, Clone, Deserialize)]
pub struct NewResource {
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub config_json: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub config_json: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

// --- Agent ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub environment_id: String,
    pub name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub ip: String,
    pub status: String,
    pub last_seen_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// --- Audit ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub time: String,
    pub action: String,
    pub target: Option<String>,
    pub environment_id: Option<String>,
    pub resource_id: Option<String>,
    pub agent_id: Option<String>,
    pub result: String,
    pub detail: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NewAuditEntry {
    pub action: String,
    pub target: Option<String>,
    pub environment_id: Option<String>,
    pub resource_id: Option<String>,
    pub agent_id: Option<String>,
    pub result: String,
    pub detail: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub time_from: Option<String>,
    pub time_to: Option<String>,
    pub action: Option<String>,
    pub environment_id: Option<String>,
    pub agent_id: Option<String>,
    pub result: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditStats {
    pub total: i64,
    pub success_count: i64,
    pub failure_count: i64,
}
