//! 资源 `config_json` 的公共读取口径（Hub 与 Agent 共用，避免键名兜底语义分叉）。

use serde_json::Value;

/// 从解密后的 config_json 读取私钥，兼容 `privateKey`（camel）与 `private_key`（snake）。
///
/// 前端写入 `private_key`，历史/Agent 侧读 `privateKey`；单键读取会让终端与
/// SFTP 拿到不同的凭据（一个落密码分支、一个只试公钥）。纯逻辑，便于单元测试。
pub fn config_private_key(config: &Value) -> Option<String> {
    config
        .get("privateKey")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            config
                .get("private_key")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_private_key_reads_snake_and_camel_keys() {
        let snake: Value = serde_json::from_str(r#"{"private_key":"PEM-SNAKE"}"#).unwrap();
        let camel: Value = serde_json::from_str(r#"{"privateKey":"PEM-CAMEL"}"#).unwrap();
        let none: Value = serde_json::from_str(r#"{"password":"x"}"#).unwrap();
        assert_eq!(
            config_private_key(&snake).as_deref(),
            Some("PEM-SNAKE"),
            "frontend writes private_key"
        );
        assert_eq!(
            config_private_key(&camel).as_deref(),
            Some("PEM-CAMEL"),
            "agent side reads privateKey"
        );
        assert_eq!(config_private_key(&none), None);
        assert_eq!(config_private_key(&Value::Null), None);
    }

    #[test]
    fn config_private_key_prefers_camel_key_when_both_present() {
        let both: Value =
            serde_json::from_str(r#"{"privateKey":"PEM-CAMEL","private_key":"PEM-SNAKE"}"#)
                .unwrap();
        assert_eq!(config_private_key(&both).as_deref(), Some("PEM-CAMEL"));
    }
}
