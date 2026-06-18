use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub id: String,
    pub created_at: String,
}

impl AgentIdentity {
    pub fn load_or_create(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join("agent.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let identity: AgentIdentity = serde_json::from_str(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            Ok(identity)
        } else {
            let id = format!("agt_{}", &Uuid::new_v4().to_string()[..8]);
            let created_at = chrono_now_iso();
            let identity = AgentIdentity { id, created_at };
            // Ensure data_dir exists
            std::fs::create_dir_all(data_dir)
                .with_context(|| format!("failed to create {}", data_dir.display()))?;
            let content = serde_json::to_string_pretty(&identity)?;
            std::fs::write(&path, content)
                .with_context(|| format!("failed to write {}", path.display()))?;
            Ok(identity)
        }
    }
}

fn chrono_now_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{now:010}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_generates_id() {
        let dir = tempfile::tempdir().unwrap();
        let identity = AgentIdentity::load_or_create(dir.path()).unwrap();
        assert!(identity.id.starts_with("agt_"));
        assert!(!identity.created_at.is_empty());
    }

    #[test]
    fn load_or_create_persists_and_loads() {
        let dir = tempfile::tempdir().unwrap();
        let identity1 = AgentIdentity::load_or_create(dir.path()).unwrap();
        let identity2 = AgentIdentity::load_or_create(dir.path()).unwrap();
        assert_eq!(identity1.id, identity2.id);
        assert_eq!(identity1.created_at, identity2.created_at);
    }

    #[test]
    fn load_or_create_creates_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("sub").join("dir");
        let identity = AgentIdentity::load_or_create(&nested).unwrap();
        assert!(identity.id.starts_with("agt_"));
        assert!(nested.exists());
    }
}
