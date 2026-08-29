//! 共享 CLI 框架 — Hub / Agent 共用 clap 子命令入口。
//!
//! 两个二进制（rex-hub / rex-agent）仅各自提供「启动逻辑」闭包，
//! 即可复用 `run` / `stop` / `version` / `service` 全部子命令。

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::service::ServiceCmd;
pub use crate::service::ServiceKind;

/// Hub / Agent 顶层 CLI。
///
/// 无子命令（直接 `rex-hub`）等价于 `run`。
///
/// 配置来源优先级：**命令行参数 > 环境变量 > 配置文件**。
/// Hub 配置文件：`~/.rex/config.yaml`；Agent 配置文件：`~/.rex/agent.yaml`。
#[derive(Parser, Debug)]
#[command(author, version, about = "REX Hub / Agent", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// 解析命令行参数（封装 `clap::Parser::parse`，调用方无需直接引入 clap trait）。
pub fn parse() -> Cli {
    Cli::parse()
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 启动服务（默认；等价于直接运行可执行文件）
    Run(RunOpts),
    /// 停止后台实例（读 pid 文件发送终止信号）
    Stop,
    /// 打印版本号（含短 git hash）
    Version,
    /// 注册 / 管理操作系统服务（登录时自动启动）
    Service(ServiceCmd),
}

/// `run` 子命令的可选参数。Hub 与 Agent 共用同一结构，各自只读取相关字段。
///
/// 配置优先级：命令行参数 > 环境变量 > 配置文件。
/// 参数通过 supervisor 透明传递给 worker 子进程。
#[derive(Args, Debug, Default)]
pub struct RunOpts {
    /// 单进程模式：直接运行业务进程，不启动 supervisor 子进程。
    /// 此模式下无法自动更新（无 supervisor 完成二进制替换），更新需手动替换二进制。
    #[arg(long)]
    pub single: bool,

    /// 后台运行：脱离终端（daemonize），日志重定向到数据目录下的 log 文件，
    /// 用 pid 文件实现单实例互斥。停止请用 `stop` 子命令。
    #[arg(long)]
    pub background: bool,

    // ── Hub 相关 ──
    /// Hub HTTP 监听端口（Hub）
    #[arg(long)]
    pub port: Option<u16>,
    /// 数据存储目录（Hub / Agent）
    #[arg(long)]
    pub data_dir: Option<std::path::PathBuf>,
    /// 前端静态资源目录（Hub）
    #[arg(long)]
    pub static_dir: Option<std::path::PathBuf>,

    // ── Agent 相关 ──
    /// Hub 的访问地址，如 http://hub.example.com:3000（Agent）
    #[arg(long)]
    pub hub_url: Option<String>,
    /// 该环境的注册令牌（Agent）
    #[arg(long)]
    pub token: Option<String>,
}

/// 分发 CLI：调用方传入自身 `run` 启动逻辑。
///
/// `Run` / `Version` / `Service` 之外的解析（如 `--help` / `--version`）由 clap 处理并自动退出。
pub fn dispatch(
    cli: Cli,
    kind: ServiceKind,
    run: impl FnOnce(&RunOpts) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    match cli.command {
        None => run(&RunOpts::default()),
        Some(Commands::Run(ref opts)) => run(opts),
        Some(Commands::Stop) => {
            let data_dir = std::env::var("REX_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".rex"));
            println!("{}", crate::process::stop(kind, &data_dir));
            Ok(())
        }
        Some(Commands::Version) => {
            println!("rex-{} {}", kind.as_str(), full_version());
            Ok(())
        }
        Some(Commands::Service(cmd)) => crate::service::handle(kind, cmd),
    }
}

/// 完整版本字符串：`0.70.8 (a1b2c3d)`（短 git hash 由 build.rs 注入）。
pub fn full_version() -> String {
    match option_env!("REX_GIT_HASH") {
        Some(hash) if !hash.is_empty() => format!("{} ({hash})", env!("CARGO_PKG_VERSION")),
        _ => env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{ServiceAction, ServiceCmd};

    #[test]
    fn test_cli_default_is_run() {
        let cli = Cli::try_parse_from(["rex-hub"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_version() {
        let cli = Cli::try_parse_from(["rex-hub", "version"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn test_cli_run_explicit() {
        let cli = Cli::try_parse_from(["rex-agent", "run"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Run(_))));
    }

    #[test]
    fn test_cli_run_single_and_background() {
        let cli = Cli::try_parse_from(["rex-hub", "run", "--single", "--background"]).unwrap();
        match cli.command {
            Some(Commands::Run(o)) => {
                assert!(o.single);
                assert!(o.background);
            }
            other => panic!("expected run, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_run_port_param() {
        let cli = Cli::try_parse_from(["rex-hub", "run", "--port", "8080"]).unwrap();
        match cli.command {
            Some(Commands::Run(o)) => assert_eq!(o.port, Some(8080)),
            other => panic!("expected run, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_stop() {
        let cli = Cli::try_parse_from(["rex-agent", "stop"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Stop)));
    }

    #[test]
    fn test_cli_service_install_system() {
        let cli = Cli::try_parse_from(["rex-hub", "service", "install", "--system"]).unwrap();
        match cli.command {
            Some(Commands::Service(ServiceCmd {
                action: ServiceAction::Install(o),
            })) => assert!(o.system),
            other => panic!("expected service install, got {other:?}"),
        }
    }

    #[test]
    fn test_full_version_format() {
        // 没有 build.rs 注入时回退到纯版本号
        let v = full_version();
        assert!(v.starts_with(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_dispatch_stop_calls_process() {
        let cli = Cli::try_parse_from(["rex-hub", "stop"]).unwrap();
        // stop 不应调用 run 闭包；这里用一个 panic 闭包验证
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dispatch(cli, ServiceKind::Hub, |_| {
                panic!("run must not be called for stop")
            })
            .unwrap();
        }));
        assert!(res.is_ok(), "stop should not invoke run closure");
    }
}
