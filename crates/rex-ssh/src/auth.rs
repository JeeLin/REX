/// SSH 认证方式
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// 密码认证
    Password(String),
    /// 公钥认证
    Key {
        private_key_path: String,
        passphrase: Option<String>,
    },
}
