# 0.7.0 代码审查报告

## 审查日期

2026-06-22

## 审查范围

`git diff de823fd..HEAD`（0.7.0 里程碑全部变更）

## 审查结果

### 🔴 必须修复（已修复）

| # | 问题 | 文件 | 状态 |
|---|------|------|------|
| 1 | HTTP-01 challenge server 绑定到主端口（3000）而非 80 端口 | `bin/rex-hub.rs:90` | ✅ 已修复：硬编码端口 80 |
| 2 | HTTP-01 server 中 `.unwrap()` 导致 bind/accept 失败时 panic | `bin/rex-hub.rs:94,97` | ✅ 已修复：改为 match + tracing::error |
| 3 | TLS-ALPN-01 challenge config 被丢弃（`_challenge_config`），IP 证书申请失败 | `bin/rex-hub.rs:82` | ✅ 已修复：使用 challenge_config |
| 4 | `rustls::ServerConfig::builder()` 缺少 CryptoProvider 导致 panic | `bin/rex-hub.rs:155`, `tls.rs:32` | ✅ 已修复：显式安装 ring provider |

### 🟡 应该修复（已修复）

| # | 问题 | 文件 | 状态 |
|---|------|------|------|
| 5 | `validate_acme_config` 定义但从未调用（死代码） | `acme.rs:10-17` | ✅ 已移除 |
| 6 | `build_server_config_from_state` 等 3 个 wrapper 函数未使用 | `acme.rs:47-65` | ✅ 已移除 |

### 🟢 可选改进（已知，暂不处理）

| # | 问题 | 文件 |
|---|------|------|
| 7 | `settings.rs` ACME cert_ready 检查依赖 `rustls-acme` 内部缓存文件名 | `settings.rs:53-55` |
| 8 | `TlsMode::SelfSigned` 现在是死代码（无配置路径可达） | `acme.rs:102` |
| 9 | `determine_tls_mode` 位于 `acme.rs` 但决定所有 TLS 模式，应在 `HubConfig` 上 | `acme.rs:77` |
| 10 | 自签名证书无过期检查或续期机制 | `self_signed.rs` |

## 结论

✅ 审查通过，所有 🔴 问题已修复。
