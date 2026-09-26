# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.89.0 | 🟢 | Agent 侧 SshHandlePool 连接池键不含 username | v0.89 步骤5审查（CR10，遗留非本版引入） | `crates/rex-agent/src/agent_ssh.rs:276`、`agent_file.rs:142` 池键为 `host:port`，与 Hub 侧 `rex_ssh::pool` 键 `user@host:port` 口径不同；同 host:port 不同用户的两个资源会共用同一条已认证会话（A 终端在线时 B 的 SFTP 以 A 身份执行）。单用户产品下属身份错用而非越权，v0.89 时点不改，另行排期 |
| v0.89.0 | 🟢 | 裸 `/api/`（尾斜杠）与 Hub 侧 `/api` 兜底缺失 | v0.89 步骤5复审（CR14，CR8 同根因残留，非修复轮引入） | ① Agent `http_server.rs:501-502` `/api/{*path}` 不匹配 `/api/`（尾斜杠，matchit catchall 空余段不匹配）→ 落 SPA 200 html；② `GET /api` 已代理到 Hub，但 Hub 无 `/api` 精确路由 → `rex-hub.rs:402` SPA fallback（`embedded_static.rs:58-67` 无扩展名回退 index.html）→ 仍 200 text/html。当前前端 `client.ts:23` 恒 `baseUrl+path`，无 `/api`、`/api/` 调用方，影响理论；修法候选：Agent 补 `.route("/api/", …)` + Hub 补 `/api`、`/api/{*path}` JSON 404 兜底 |
| v0.89.0 | 🟡 | agent 模式 SSH/SFTP「测试连接」以空用户名认证 | v0.89 步骤5复审（CR15，范围外既有，最后改动 v0.86） | `resource_api.rs:297-300` `TestConnectionRequest.username` 标 `#[allow(dead_code)]`，agent 分支 `connect_config`（`:396-411`）只含 host/port + config_json，不透出顶层 username → agent 环境对 SSH/SFTP 资源点「测试连接」以空用户名认证必拒（与 CR12 同源入口但独立：CR12 修数据面 config 下发，此条是测试连接入口）。`8d3255a..c7839d6` 全程零改动，另行排期 |
