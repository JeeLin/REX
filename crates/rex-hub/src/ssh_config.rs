use serde::{Deserialize, Serialize};

/// SSH 认证配置（来自前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshAuthConfig {
    /// 认证类型："password" | "key"
    #[serde(rename = "type")]
    pub auth_type: String,
    /// 密码（明文，前端提交）
    #[serde(default)]
    pub password: Option<String>,
    /// 加密后密码（存库）
    #[serde(default)]
    pub password_encrypted: Option<String>,
    /// 密钥路径
    #[serde(default)]
    pub private_key_path: Option<String>,
    /// 密钥密码（明文）
    #[serde(default)]
    pub passphrase: Option<String>,
    /// 加密后密钥密码
    #[serde(default)]
    pub passphrase_encrypted: Option<String>,
}

/// SSH 终端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshTerminalConfig {
    /// 字符编码，默认 "utf-8"
    #[serde(default = "default_encoding")]
    pub encoding: String,
    /// 保持连接间隔秒数，默认 60
    #[serde(default = "default_keep_alive")]
    pub keep_alive_seconds: u32,
}

impl Default for SshTerminalConfig {
    fn default() -> Self {
        Self {
            encoding: default_encoding(),
            keep_alive_seconds: default_keep_alive(),
        }
    }
}

fn default_encoding() -> String {
    "utf-8".to_string()
}

fn default_keep_alive() -> u32 {
    60
}

/// SSH 资源完整配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshResourceConfig {
    /// 服务器地址（必填）
    pub host: String,
    /// 端口，默认 22
    #[serde(default = "default_port")]
    pub port: u16,
    /// 用户名（必填）
    pub username: String,
    /// 认证配置
    pub auth: SshAuthConfig,
    /// 终端配置
    #[serde(default)]
    pub terminal: SshTerminalConfig,
}

fn default_port() -> u16 {
    22
}

impl SshResourceConfig {
    /// 从 JSON 字符串解析并验证 SSH 配置
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let config: SshResourceConfig =
            serde_json::from_str(json).map_err(|e| anyhow::anyhow!("invalid JSON: {}", e))?;

        config.validate()?;
        Ok(config)
    }

    /// 验证配置字段
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.host.is_empty() {
            anyhow::bail!("host is required");
        }
        if self.username.is_empty() {
            anyhow::bail!("username is required");
        }
        if self.port == 0 {
            anyhow::bail!("invalid port: {}", self.port);
        }
        match self.auth.auth_type.as_str() {
            "password" => {
                if self.auth.password.is_none() && self.auth.password_encrypted.is_none() {
                    anyhow::bail!("password is required for password auth");
                }
            }
            "key" => {
                if self.auth.private_key_path.is_none() {
                    anyhow::bail!("private_key_path is required for key auth");
                }
            }
            other => {
                anyhow::bail!("unsupported auth type: {}", other);
            }
        }
        Ok(())
    }

    /// 将敏感字段加密，返回加密后的 JSON 字符串
    pub fn encrypt_sensitive(&self, secret_key: &str) -> anyhow::Result<String> {
        let mut config = self.clone();
        if let Some(ref password) = config.auth.password {
            config.auth.password_encrypted = Some(rex_ssh::crypto::encrypt(password, secret_key));
            config.auth.password = None;
        }
        if let Some(ref passphrase) = config.auth.passphrase {
            config.auth.passphrase_encrypted =
                Some(rex_ssh::crypto::encrypt(passphrase, secret_key));
            config.auth.passphrase = None;
        }
        serde_json::to_string(&config).map_err(|e| anyhow::anyhow!("serialize failed: {}", e))
    }

    /// 从存库的 JSON 解析，解密敏感字段
    pub fn from_encrypted_json(json: &str, secret_key: &str) -> anyhow::Result<Self> {
        let mut config: SshResourceConfig =
            serde_json::from_str(json).map_err(|e| anyhow::anyhow!("invalid JSON: {}", e))?;

        if let Some(ref encrypted) = config.auth.password_encrypted {
            config.auth.password = Some(rex_ssh::crypto::decrypt(encrypted, secret_key)?);
        }
        if let Some(ref encrypted) = config.auth.passphrase_encrypted {
            config.auth.passphrase = Some(rex_ssh::crypto::decrypt(encrypted, secret_key)?);
        }
        config.validate()?;
        Ok(config)
    }

    /// 转换为 rex_ssh::AuthMethod
    pub fn to_auth_method(&self, secret_key: &str) -> anyhow::Result<rex_ssh::auth::AuthMethod> {
        match self.auth.auth_type.as_str() {
            "password" => {
                let password = self
                    .auth
                    .password
                    .clone()
                    .or_else(|| {
                        self.auth
                            .password_encrypted
                            .as_ref()
                            .and_then(|e| rex_ssh::crypto::decrypt(e, secret_key).ok())
                    })
                    .ok_or_else(|| anyhow::anyhow!("no password available"))?;
                Ok(rex_ssh::auth::AuthMethod::Password(password))
            }
            "key" => {
                let path = self
                    .auth
                    .private_key_path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("private_key_path is required"))?
                    .clone();
                let passphrase = self.auth.passphrase.clone().or_else(|| {
                    self.auth
                        .passphrase_encrypted
                        .as_ref()
                        .and_then(|e| rex_ssh::crypto::decrypt(e, secret_key).ok())
                });
                Ok(rex_ssh::auth::AuthMethod::Key {
                    private_key_path: path,
                    passphrase,
                })
            }
            _ => anyhow::bail!("unsupported auth type: {}", self.auth.auth_type),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_password_config() -> &'static str {
        r#"{
            "host": "192.168.1.1",
            "port": 22,
            "username": "root",
            "auth": {
                "type": "password",
                "password": "secret123"
            }
        }"#
    }

    fn valid_key_config() -> &'static str {
        r#"{
            "host": "192.168.1.1",
            "username": "root",
            "auth": {
                "type": "key",
                "private_key_path": "/home/user/.ssh/id_rsa"
            }
        }"#
    }

    #[test]
    fn parse_valid_password_config() {
        let config = SshResourceConfig::from_json(valid_password_config()).unwrap();
        assert_eq!(config.host, "192.168.1.1");
        assert_eq!(config.port, 22);
        assert_eq!(config.username, "root");
        assert_eq!(config.auth.auth_type, "password");
        assert_eq!(config.auth.password.as_deref(), Some("secret123"));
    }

    #[test]
    fn parse_valid_key_config() {
        let config = SshResourceConfig::from_json(valid_key_config()).unwrap();
        assert_eq!(config.auth.auth_type, "key");
        assert_eq!(
            config.auth.private_key_path.as_deref(),
            Some("/home/user/.ssh/id_rsa")
        );
    }

    #[test]
    fn parse_missing_host() {
        let json = r#"{"username": "root", "auth": {"type": "password", "password": "x"}}"#;
        assert!(SshResourceConfig::from_json(json).is_err());
    }

    #[test]
    fn parse_missing_username() {
        let json = r#"{"host": "1.2.3.4", "auth": {"type": "password", "password": "x"}}"#;
        assert!(SshResourceConfig::from_json(json).is_err());
    }

    #[test]
    fn parse_invalid_auth_type() {
        let json = r#"{"host": "1.2.3.4", "username": "root", "auth": {"type": "oauth"}}"#;
        assert!(SshResourceConfig::from_json(json).is_err());
    }

    #[test]
    fn parse_invalid_json() {
        assert!(SshResourceConfig::from_json("not json").is_err());
    }

    #[test]
    fn default_port_is_22() {
        let config = SshResourceConfig::from_json(valid_key_config()).unwrap();
        assert_eq!(config.port, 22);
    }

    #[test]
    fn encrypt_sensitive_fields() {
        let secret = "test-secret";
        let config = SshResourceConfig::from_json(valid_password_config()).unwrap();
        let encrypted_json = config.encrypt_sensitive(secret).unwrap();
        assert!(encrypted_json.contains("password_encrypted"));
        assert!(!encrypted_json.contains("secret123"));
    }

    #[test]
    fn decrypt_restores_original() {
        let secret = "test-secret";
        let config = SshResourceConfig::from_json(valid_password_config()).unwrap();
        let encrypted_json = config.encrypt_sensitive(secret).unwrap();
        let restored = SshResourceConfig::from_encrypted_json(&encrypted_json, secret).unwrap();
        assert_eq!(restored.auth.password.as_deref(), Some("secret123"));
        assert_eq!(restored.host, "192.168.1.1");
    }

    #[test]
    fn to_auth_method_password() {
        let secret = "test-secret";
        let config = SshResourceConfig::from_json(valid_password_config()).unwrap();
        let auth = config.to_auth_method(secret).unwrap();
        match auth {
            rex_ssh::auth::AuthMethod::Password(p) => assert_eq!(p, "secret123"),
            _ => panic!("expected Password"),
        }
    }

    #[test]
    fn to_auth_method_key() {
        let secret = "test-secret";
        let config = SshResourceConfig::from_json(valid_key_config()).unwrap();
        let auth = config.to_auth_method(secret).unwrap();
        match auth {
            rex_ssh::auth::AuthMethod::Key {
                private_key_path, ..
            } => {
                assert_eq!(private_key_path, "/home/user/.ssh/id_rsa");
            }
            _ => panic!("expected Key"),
        }
    }

    #[test]
    fn to_auth_method_key_with_passphrase() {
        let json = r#"{
            "host": "1.2.3.4",
            "username": "root",
            "auth": {
                "type": "key",
                "private_key_path": "/key",
                "passphrase": "mypass"
            }
        }"#;
        let config = SshResourceConfig::from_json(json).unwrap();
        let auth = config.to_auth_method("secret").unwrap();
        match auth {
            rex_ssh::auth::AuthMethod::Key { passphrase, .. } => {
                assert_eq!(passphrase.as_deref(), Some("mypass"));
            }
            _ => panic!("expected Key"),
        }
    }

    #[test]
    fn empty_host_rejected() {
        let json =
            r#"{"host": "", "username": "root", "auth": {"type": "password", "password": "x"}}"#;
        assert!(SshResourceConfig::from_json(json).is_err());
    }

    #[test]
    fn empty_username_rejected() {
        let json =
            r#"{"host": "1.2.3.4", "username": "", "auth": {"type": "password", "password": "x"}}"#;
        assert!(SshResourceConfig::from_json(json).is_err());
    }

    #[test]
    fn invalid_port_rejected() {
        let json = r#"{"host": "1.2.3.4", "port": 0, "username": "root", "auth": {"type": "password", "password": "x"}}"#;
        assert!(SshResourceConfig::from_json(json).is_err());
    }

    #[test]
    fn terminal_config_defaults() {
        let json = r#"{"host": "1.2.3.4", "username": "root", "auth": {"type": "password", "password": "x"}}"#;
        let config = SshResourceConfig::from_json(json).unwrap();
        assert_eq!(config.terminal.encoding, "utf-8");
        assert_eq!(config.terminal.keep_alive_seconds, 60);
    }
}
