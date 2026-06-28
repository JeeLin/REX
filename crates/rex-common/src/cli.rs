use clap::Parser;

/// REX Hub / Agent 共享 CLI 参数
#[derive(Parser)]
#[command(name = "rex", version)]
pub struct Cli {
    /// 以 worker 模式运行
    #[arg(long)]
    pub worker: bool,

    /// 配置文件路径（仅 Hub）
    #[arg(long)]
    pub config: Option<String>,

    /// TLS 证书文件路径（仅 Hub）
    #[arg(long = "tls-cert")]
    pub tls_cert: Option<String>,

    /// TLS 私钥文件路径（仅 Hub）
    #[arg(long = "tls-key")]
    pub tls_key: Option<String>,

    /// ACME 域名或公网 IP（仅 Hub）
    #[arg(long = "acme-domain")]
    pub acme_domain: Option<String>,

    /// ACME Let's Encrypt 账户邮箱（仅 Hub）
    #[arg(long = "acme-email")]
    pub acme_email: Option<String>,

    /// ACME 使用 staging 环境（仅 Hub）
    #[arg(long = "acme-staging")]
    pub acme_staging: bool,

    /// CA 证书文件路径（PEM 格式，仅 Agent）
    #[arg(long = "ca-cert")]
    pub ca_cert: Option<String>,

    /// 跳过 TLS 证书验证（仅开发/测试环境，仅 Agent）
    #[arg(long = "insecure")]
    pub insecure: bool,

    /// HTTP-01 challenge 服务器端口（仅 Hub）
    #[arg(long = "acme-http-port")]
    pub acme_http_port: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_struct_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_worker_flag() {
        let cli = Cli::parse_from(["rex", "--worker"]);
        assert!(cli.worker);
        assert!(cli.config.is_none());
    }

    #[test]
    fn parse_no_flags() {
        let cli = Cli::parse_from(["rex"]);
        assert!(!cli.worker);
    }

    #[test]
    fn parse_config_flag() {
        let cli = Cli::parse_from(["rex", "--worker", "--config", "hub.yaml"]);
        assert!(cli.worker);
        assert_eq!(cli.config.as_deref(), Some("hub.yaml"));
    }

    #[test]
    fn parse_acme_flags() {
        let cli = Cli::parse_from([
            "rex",
            "--worker",
            "--acme-domain",
            "hub.example.com",
            "--acme-email",
            "admin@example.com",
            "--acme-staging",
        ]);
        assert_eq!(cli.acme_domain.as_deref(), Some("hub.example.com"));
        assert_eq!(cli.acme_email.as_deref(), Some("admin@example.com"));
        assert!(cli.acme_staging);
    }
}
