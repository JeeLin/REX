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
}
