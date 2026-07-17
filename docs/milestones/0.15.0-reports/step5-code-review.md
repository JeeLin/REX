# Step 5: 代码审查

## 审查范围

M14 子任务1-5的全部变更 + 运行时发现的 auth middleware 修复。

## 审查维度

### 1. 正确性

| 发现 | 严重度 | 说明 |
|------|--------|------|
| Auth middleware 修复 | 🟢 | `FromRequestParts<AppState>` + `from_extractor_with_state` 确保 state 正确传入，修复了之前 `parts.extensions.get::<AppState>()` 可能失败的问题 |
| 中文错误消息 → 英文 | 🟢 | auth.rs 中 `密码错误` → `invalid password`，`密码已设置` → `password already set`，与前端 i18n 一致 |
| i18n 翻译完整 | 🟢 | zh/en 结构对称，所有页面 $t() 调用正确 |
| 加密 roundtrip | 🟢 | 4个单元测试验证正确性 |
| 主题持久化 | 🟢 | localStorage + 后端 API 双重持久化，App.vue 早期恢复 |
| 底部导航 | 🟢 | RouterLink 正确绑定路由 |

### 2. 安全性

| 发现 | 严重度 | 说明 |
|------|--------|------|
| master key 文件权限 | 🟡 | `.master-key` 文件未设置 600 权限（建议修复，自托管低风险） |
| 加密密钥管理 | 🟢 | 使用 OsRng 生成随机密钥，存储在数据目录 |
| config_json 加密 | 🟢 | 存储前加密，读取后解密，数据库中无明文密码 |
| Auth middleware state 注入 | 🟢 | 使用 `from_extractor_with_state` 确保 state 正确传入，不再依赖 extensions |

### 3. 架构一致性

| 发现 | 严重度 | 说明 |
|------|--------|------|
| crypto 集成方式 | 🟢 | 通过 AppState 注入，与其他组件一致 |
| i18n 使用方式 | 🟢 | useI18n() + t()，与 AppLayout 已有模式一致 |
| 响应式方案 | 🟢 | CSS media query，与现有 MobileTerminalBar 一致 |
| AuthUser 实现 | 🟢 | 使用 `FromRequestParts<AppState>` 具体类型，与路由状态绑定一致 |

### 4. 测试覆盖

| 发现 | 严重度 | 说明 |
|------|--------|------|
| crypto 测试 | 🟢 | 4个测试覆盖核心路径 |
| auth 测试 | 🟢 | 3个测试覆盖 JWT + 密码 |

### 5. 与里程碑文档一致性

| 检查项 | 结果 |
|--------|------|
| 子任务1 i18n | ✅ 所有页面使用 $t() |
| 子任务2 主题持久化 | ✅ localStorage + 后端 API |
| 子任务3 响应式 | ✅ 底部导航 + 页面自适应 |
| 子任务4 凭据加密 | ✅ AES-256-GCM |
| 子任务5 测试 | ✅ crypto + auth 测试 |

## 结论

✅ 无 🔴 必须修复项。1个 🟡 建议修复（master key 文件权限）。

## 变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| middleware.rs | 功能修复 | AuthUser 从 extensions 获取 state → 直接使用 state 参数 |
| rex-hub.rs | 功能修复 | from_extractor → from_extractor_with_state |
| auth.rs | 国际化 | 中文错误消息 → 英文 |
| agent_api.rs | 格式化 | cargo fmt |
| audit_api.rs | 格式化 | cargo fmt |
| dashboard_api.rs | 格式化 | cargo fmt |
| settings_api.rs | 格式化 | cargo fmt |
| sql_api.rs | 格式化 | cargo fmt |
| crypto.rs | 格式化 | cargo fmt |
