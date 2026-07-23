# M38: 测试覆盖 + Lint 清理

## Context

M37 完成了 i18n 完整翻译、性能优化和生产加固。代码库功能完整，但存在两个显著质量问题：
1. **零测试覆盖**：Rust 10 个 crate 和前端 Vue 组件均无任何单元测试
2. **前端 lint warnings**：138 个 `vue/attributes-order` warning（`title` 属性应在 `@click` 之前）

本里程碑补全关键模块测试覆盖，清理 lint warnings，提升代码质量基线。

版本类型：patch（无新功能，纯质量改善），版本号 0.35.0 → 0.35.1。

## 产品边界

**本阶段做：**
- Rust 核心模块单元测试（auth、db、middleware、error）
- 前端关键模块单元测试（stores、composables、utils）
- 前端 138 个 lint warnings 清理
- `cargo clippy` + `bun run lint` 全绿

**本阶段不做：**
- 新功能开发
- 集成测试 / E2E 测试
- 测试覆盖率工具集成（如 tarpaulin）
- 性能基准测试

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Rust 测试：auth 模块（JWT 签发/验证、密码哈希） | ⬜ |
| 2 | Rust 测试：db 模块（CRUD 操作、迁移） | ⬜ |
| 3 | Rust 测试：middleware 模块（认证中间件、请求日志） | ⬜ |
| 4 | Rust 测试：error 模块 + 公共类型 | ⬜ |
| 5 | 前端测试：stores（auth、environments、resources） | ⬜ |
| 6 | 前端测试：composables + utils | ⬜ |
| 7 | 前端 lint warnings 清理（138 个 vue/attributes-order） | ⬜ |

## 子任务详细设计

### 1 Rust 测试：auth 模块

**功能目标**

为 auth 模块的公共 API 编写单元测试：
- `hash_password` / `verify_password` — 密码哈希与验证
- `generate_token` / `verify_token` — JWT 签发与验证
- Token 过期处理
- 无效 token 拒绝

**文件结构**

修改：
- `crates/rex-hub/src/auth.rs` — 添加 `#[cfg(test)] mod tests`

**测试用例**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let hash = hash_password("test123");
        assert!(verify_password("test123", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn test_generate_and_verify_token() {
        let secret = b"test-secret-key";
        let token = generate_token(secret, "user");
        let claims = verify_token(&token, secret);
        assert!(claims.is_ok());
    }

    #[test]
    fn test_invalid_token_rejected() {
        let secret = b"test-secret-key";
        let result = verify_token("invalid.token.here", secret);
        assert!(result.is_err());
    }
}
```

**测试标准**

- 所有测试通过
- 覆盖 auth 模块的公共函数

**提交信息**: `test(auth): add unit tests for password hashing and JWT`

### 2 Rust 测试：db 模块

**功能目标**

为 db 模块的 CRUD 操作编写单元测试：
- 环境创建/读取/更新/删除
- 资源创建/读取/更新/删除
- Agent 创建/读取/更新
- 审计日志写入
- 设置读写

**文件结构**

修改：
- `crates/rex-hub/src/db.rs` — 添加 `#[cfg(test)] mod tests`

**测试用例**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::new(":memory:").unwrap()
    }

    #[test]
    fn test_create_and_get_environment() {
        let db = test_db();
        let env = db.create_environment("test", "desc", "direct").unwrap();
        let got = db.get_environment(&env.id).unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().name, "test");
    }

    #[test]
    fn test_delete_environment() {
        let db = test_db();
        let env = db.create_environment("test", "desc", "direct").unwrap();
        db.delete_environment(&env.id).unwrap();
        assert!(db.get_environment(&env.id).unwrap().is_none());
    }

    // ... 更多 CRUD 测试
}
```

**测试标准**

- 所有 CRUD 操作有正向和反向测试
- 使用内存数据库（`:memory:`）避免磁盘 IO

**提交信息**: `test(db): add unit tests for environment and resource CRUD`

### 3 Rust 测试：middleware 模块

**功能目标**

为 middleware 模块编写单元测试：
- `AuthUser` 提取器（有效 token / 无效 token / 缺失 token）
- `request_logger` 中间件（静态文件跳过、API 请求记录）
- `security_headers` 中间件（响应头注入）

**文件结构**

修改：
- `crates/rex-hub/src/middleware.rs` — 添加 `#[cfg(test)] mod tests`

**测试用例**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_valid_token() {
        // 创建有效 token → 提取成功
    }

    #[test]
    fn test_auth_user_invalid_token() {
        // 创建无效 token → 返回 401
    }

    #[test]
    fn test_auth_user_missing_token() {
        // 无 token → 返回 401
    }
}
```

**测试标准**

- 认证逻辑覆盖完整
- 安全头注入验证

**提交信息**: `test(middleware): add unit tests for auth extractor and security headers`

### 4 Rust 测试：error 模块 + 公共类型

**功能目标**

为 error 模块和 rex-common 的公共类型编写测试：
- 错误类型创建与转换
- 错误响应格式
- 公共类型序列化/反序列化

**文件结构**

修改：
- `crates/rex-common/src/error.rs` — 添加测试
- `crates/rex-common/src/types.rs` — 添加测试（如有）

**测试标准**

- 错误类型覆盖所有变体
- 序列化/反序列化正确

**提交信息**: `test(common): add unit tests for error types and public models`

### 5 前端测试：stores

**功能目标**

为 Pinia stores 编写单元测试：
- `authStore` — 登录/登出/token 管理
- `environmentsStore` — 环境列表/CRUD
- `resourcesStore`（如有）— 资源列表/CRUD

**文件结构**

创建：
- `packages/rex-console-web/src/stores/__tests__/auth.test.ts`
- `packages/rex-console-web/src/stores/__tests__/environments.test.ts`

**测试标准**

- Store 状态变更正确
- API 调用 mock 正确
- 错误处理覆盖

**提交信息**: `test(web): add unit tests for auth and environments stores`

### 6 前端测试：composables + utils

**功能目标**

为 composables 和 utils 编写测试：
- `useKeyboardShortcuts` — 快捷键注册/注销
- 工具函数（如有）

**文件结构**

创建：
- `packages/rex-console-web/src/composables/__tests__/useKeyboardShortcuts.test.ts`

**测试标准**

- 快捷键注册/注销正确
- 键盘事件处理正确

**提交信息**: `test(web): add unit tests for keyboard shortcuts composable`

### 7 前端 lint warnings 清理

**功能目标**

修复 138 个 `vue/attributes-order` warnings。这些 warning 是因为 `:title` 绑定属性出现在 `@click` 事件之后，Vue 风格指南要求静态属性在前、指令属性在后。

**文件结构**

修改：所有出现 warning 的 `.vue` 文件

**修复方式**

```vue
<!-- Before (warning) -->
<button @click="handler" :title="t('xxx')">Click</button>

<!-- After (fixed) -->
<button :title="t('xxx')" @click="handler">Click</button>
```

或使用 `--fix` 自动修复：
```bash
cd packages/rex-console-web && bun run lint --fix
```

**测试标准**

- `bun run lint` 输出 0 warnings
- `bun run type-check` 通过
- `bun run build` 通过

**提交信息**: `style(web): fix vue/attributes-order lint warnings`

## 设计核对点

- ✅ 不引入新功能
- ✅ 测试使用内存数据库，不依赖外部服务
- ✅ 前端测试 mock API 调用
- ✅ lint 修复不改变功能行为
- ✅ 所有测试在 CI 中可运行

## Flow Status

- [x] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
