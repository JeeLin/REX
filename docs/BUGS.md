# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.89.0 | 🟢 | Agent 侧 SshHandlePool 连接池键不含 username | v0.89 步骤5审查（CR10，遗留非本版引入） | `crates/rex-agent/src/agent_ssh.rs:276`、`agent_file.rs:142` 池键为 `host:port`，与 Hub 侧 `rex_ssh::pool` 键 `user@host:port` 口径不同；同 host:port 不同用户的两个资源会共用同一条已认证会话（A 终端在线时 B 的 SFTP 以 A 身份执行）。单用户产品下属身份错用而非越权，v0.89 时点不改，另行排期 |
|----------|--------|------|------|------|
