# M0: 项目骨架

## Context

M0 是整个项目的起点。建立 Rust workspace 骨架和 Vue 3 前端工程，为后续所有里程碑提供可编译、可运行的基础。本里程碑不实现任何业务逻辑，只交付工程骨架和进程模型。

## 产品边界

**做什么：**
- Rust workspace 骨架（rex-common、rex-binaries）
- Hub / Agent 二进制入口 + CLI 参数解析
- supervisor + worker 进程模型（第一阶段：启动、监控、重启）
- 基础 tracing 日志
- Vue 3 + Vite 前端工程初始化
- 前端目录结构、路由骨架、主题系统、i18n 基础
- 布局组件骨架（AppLayout、FullScreenLayout）

**不做什么：**
- HTTP API（M1）
- SQLite 数据库（M1）
- 登录认证（M1）
- Agent 注册 / 心跳（M2）
- SSH / SFTP / SQL / 文件传输协议（M3-M6）
- 自动更新（M7-M8）
- 前端业务页面（M1 起逐步实现）

## 子任务清单

| 子任务 | 内容 | 前端/后端 | 状态 |
|--------|------|-----------|------|
| 0.1 | Rust workspace 骨架 | 后端 | ⬜ |
| 0.2 | Hub 二进制入口 + supervisor/worker | 后端 | ⬜ |
| 0.3 | Agent 二进制入口 + supervisor/worker | 后端 | ⬜ |
| 0.4 | Vue 3 前端工程初始化 | 前端 | ⬜ |
| 0.5 | 主题系统 + i18n 基础 | 前端 | ⬜ |
| 0.6 | 布局组件骨架 | 前端 | ⬜ |

---

## 子任务 0.1：Rust workspace 骨架

### 功能目标

建立 Rust workspace，创建 rex-common、rex-hub、rex-agent 三个 crate，声明共享依赖版本。

### 文件结构

```text
.
├── Cargo.toml                    workspace 根配置
├── crates/
│   ├── rex-common/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── rex-hub/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   └── rex-agent/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
```

### 接口设计

根 `Cargo.toml`：

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
```

`rex-common/Cargo.toml`：

```toml
[package]
name = "rex-common"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
anyhow = { workspace = true }
```

`rex-hub/Cargo.toml` 和 `rex-agent/Cargo.toml`：

```toml
[package]
name = "rex-hub"  # 或 rex-agent
version = "0.1.0"
edition = "2021"

[dependencies]
rex-common = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
clap = { workspace = true }
```

`rex-common/src/lib.rs`：空模块。

`rex-hub/src/main.rs` 和 `rex-agent/src/main.rs`：最小入口，打印名称后退出。

### 后端流程

```text
cargo run -p rex-hub → 打印 "rex-hub" 并退出
cargo run -p rex-agent → 打印 "rex-agent" 并退出
```

### 测试 / QA 标准

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p rex-hub
cargo run -p rex-agent
```

验证点：
- workspace 可编译
- 三个 crate 各自可编译
- 子 crate 使用 `workspace = true` 声明依赖
- `cargo fmt` / `clippy` / `test` 全部通过

### 提交边界

```text
chore: add Rust workspace skeleton
```

提交内容：根 Cargo.toml、crates/rex-common、crates/rex-hub、crates/rex-agent。

---

## 子任务 0.2：Hub 二进制入口 + supervisor/worker

### 功能目标

Hub 二进制实现 CLI 参数解析和 supervisor + worker 进程模型。supervisor 启动 worker 子进程，监控退出并重启。worker 执行业务逻辑（本阶段只打印日志并退出）。

### 文件结构

```text
crates/rex-common/src/
├── lib.rs          修改：导出 cli 和 supervisor 模块
├── cli.rs          新增：CLI 参数解析
└── supervisor.rs   新增：supervisor 逻辑

crates/rex-hub/src/
└── main.rs         修改：区分 supervisor/worker 模式
```

### 接口设计

```rust
// rex-common/src/cli.rs
use clap::Parser;

#[derive(Parser)]
#[command(name = "rex")]
pub struct Cli {
    /// 运行在 worker 模式
    #[arg(long)]
    pub worker: bool,
}
```

```rust
// rex-common/src/supervisor.rs
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct SupervisorConfig {
    pub restart_delay: Duration,
}

pub fn run_supervisor(config: SupervisorConfig) -> anyhow::Result<()> {
    loop {
        let mut child = Command::new(std::env::current_exe()?)
            .arg("--worker")
            .spawn()?;

        let status = child.wait()?;
        let code = status.code().unwrap_or(1);

        tracing::info!(exit_code = code, "worker exited");

        if code != 0 {
            tracing::warn!(delay_ms = config.restart_delay.as_millis() as u64, "restarting worker");
            thread::sleep(config.restart_delay);
        } else {
            // exit 0 = 正常退出，不再重启
            break;
        }
    }
    Ok(())
}
```

### 后端流程

```text
rex-hub
  ↓
parse CLI (--worker?)
  ├─ yes → run_hub_worker() → 打印日志 → exit 0
  └─ no  → run_supervisor(SupervisorConfig { restart_delay: 1s })
             ↓
           spawn worker --worker
             ↓
           wait exit
             ↓
           exit 0 → break（不再重启）
           exit ≠ 0 → sleep → restart
```

### 测试 / QA 标准

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo run -p rex-hub          # supervisor 模式，启动 worker
cargo run -p rex-hub -- --worker  # worker 模式，打印日志后退出
```

验证点：
- supervisor 模式启动 worker 并等待退出
- worker exit 0 → supervisor 退出
- worker exit 非 0 → supervisor 重启
- SIGTERM 停止 supervisor
- 日志输出正确

### 提交边界

```text
feat: add CLI parsing and supervisor/worker process model
```

---

## 子任务 0.3：Agent 二进制入口 + supervisor/worker

### 功能目标

Agent 二进制复用 rex-common 的 CLI 和 supervisor，实现独立的 worker 入口。

### 文件结构

```text
crates/rex-agent/src/
└── main.rs         修改：区分 supervisor/worker 模式
```

### 后端流程

与 Hub 完全一致，worker 打印 "rex-agent worker started" 后退出。

### 测试 / QA 标准

```bash
cargo run -p rex-agent         # supervisor 模式
cargo run -p rex-agent -- --worker  # worker 模式
```

### 提交边界

```text
feat: add Agent binary with supervisor/worker
```

---

## 子任务 0.4：Vue 3 前端工程初始化

### 功能目标

初始化 Vue 3 + Vite + TypeScript 前端工程，建立目录结构、路由骨架、基础构建配置。

### 文件结构

```text
packages/rex-console-web/
├── package.json
├── index.html
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── env.d.ts
└── src/
    ├── main.ts
    ├── App.vue
    ├── router.ts
    ├── api/                    空目录（M1 填充）
    ├── stores/                 空目录（M1 填充）
    ├── features/               空目录（M3 起填充）
    ├── pages/
    │   ├── Login.vue           占位：登录页
    │   └── Dashboard.vue       占位：仪表盘
    ├── components/             空目录（M1 起填充）
    ├── layouts/
    │   ├── AppLayout.vue       占位：标准布局
    │   └── FullScreenLayout.vue 占位：全屏布局
    ├── styles/
    │   ├── variables.css       从 prototype/css/variables.css 迁移
    │   └── base.css            基础重置样式
    └── i18n/
        ├── index.ts            i18n 初始化
        ├── zh.ts               中文翻译（基础 key）
        └── en.ts               英文翻译（基础 key）
```

### 接口设计

`package.json`：

```json
{
  "name": "rex-console-web",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc -b && vite build",
    "preview": "vite preview",
    "type-check": "vue-tsc --noEmit",
    "lint": "eslint . --ext .vue,.ts"
  },
  "dependencies": {
    "vue": "^3.5.0",
    "vue-router": "^4.4.0",
    "pinia": "^2.2.0",
    "vue-i18n": "^10.0.0"
  },
  "devDependencies": {
    "@vitejs/plugin-vue": "^5.1.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0",
    "vue-tsc": "^2.1.0",
    "eslint": "^9.0.0"
  }
}
```

`vite.config.ts`：

```ts
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:3000',
    },
  },
  build: {
    outDir: 'dist',
  },
})
```

`src/router.ts`：

```ts
import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/login', name: 'login', component: () => import('./pages/Login.vue') },
    { path: '/', name: 'dashboard', component: () => import('./pages/Dashboard.vue') },
  ],
})

export default router
```

`src/main.ts`：

```ts
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { i18n } from './i18n'
import './styles/variables.css'
import './styles/base.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)
app.mount('#app')
```

### 前端交互

M0 只搭建骨架，不实现具体交互。路由占位页面显示页面名称即可。

### 测试 / QA 标准

```bash
cd packages/rex-console-web
npm install
npm run type-check
npm run build
npm run dev  # 手动验证页面加载
```

验证点：
- `npm install` 无报错
- `npm run type-check` 通过
- `npm run build` 成功
- `npm run dev` 启动后访问 http://localhost:5173 显示占位页面
- 路由切换正常

### 提交边界

```text
feat: initialize Vue 3 frontend with Vite and routing
```

---

## 子任务 0.5：主题系统 + i18n 基础

### 功能目标

从 `prototype/css/variables.css` 迁移设计 token 到前端工程，实现深色/浅色/跟随系统主题切换。建立 i18n 基础框架。

### 文件结构

```text
packages/rex-console-web/src/
├── styles/
│   ├── variables.css     从 prototype 迁移（深色 + 浅色主题变量）
│   └── base.css          全局重置、字体引入
├── i18n/
│   ├── index.ts          vue-i18n 配置
│   ├── zh.ts             中文翻译
│   └── en.ts             英文翻译
└── stores/
    └── user.ts           主题、语言偏好（localStorage 持久化）
```

### 接口设计

主题变量从 `prototype/css/variables.css` 迁移，使用 CSS 类切换：

```css
/* 深色主题（默认） */
:root { /* 变量值 */ }

/* 浅色主题 */
:root.theme-light { /* 覆盖变量值 */ }

/* 跟随系统 */
@media (prefers-color-scheme: light) {
  :root:not(.theme-dark):not(.theme-light) { /* 覆盖变量值 */ }
}
```

i18n 使用 vue-i18n：

```ts
// i18n/index.ts
import { createI18n } from 'vue-i18n'
import zh from './zh'
import en from './en'

export const i18n = createI18n({
  legacy: false,
  locale: localStorage.getItem('rex-lang') || 'zh',
  fallbackLocale: 'en',
  messages: { zh, en },
})
```

翻译 key 基础结构：

```ts
// i18n/zh.ts
export default {
  app: { name: 'REX Hub' },
  common: { loading: '加载中', error: '错误', confirm: '确认', cancel: '取消' },
  nav: { dashboard: '仪表盘', environments: '环境', agents: 'Agent', settings: '设置' },
  auth: { login: '登录', logout: '退出登录' },
}
```

### 前端交互

- 页面加载时读取 localStorage 中的主题和语言偏好
- 默认深色主题
- `prefers-color-scheme` 媒体查询用于跟随系统模式

### 测试 / QA 标准

```bash
cd packages/rex-console-web
npm run type-check
npm run build
```

验证点：
- CSS 变量正确加载
- 深色/浅色主题可切换
- i18n 切换中英文生效
- localStorage 持久化偏好

### 提交边界

```text
feat: add theme system and i18n foundation
```

---

## 子任务 0.6：布局组件骨架

### 功能目标

实现 AppLayout（标准布局：侧边栏 + 主内容区）和 FullScreenLayout（全屏布局）的骨架组件。

### 文件结构

```text
packages/rex-console-web/src/layouts/
├── AppLayout.vue         标准布局：侧边栏 + 顶栏 + 主内容区
└── FullScreenLayout.vue  全屏布局：只含 router-view
```

### 接口设计

```vue
<!-- AppLayout.vue -->
<template>
  <div class="app-layout">
    <aside class="sidebar"><!-- 侧边栏占位 --></aside>
    <main class="content">
      <header class="topbar"><!-- 顶栏占位 --></header>
      <router-view />
    </main>
  </div>
</template>
```

```vue
<!-- FullScreenLayout.vue -->
<template>
  <div class="fullscreen-layout">
    <router-view />
  </div>
</template>
```

### 前端交互

侧边栏和顶栏在 M0 只显示占位文字，M1 填充具体内容。

### 测试 / QA 标准

```bash
cd packages/rex-console-web
npm run type-check
npm run build
```

验证点：
- 布局组件可渲染
- router-view 正确嵌套
- 无 TypeScript 错误

### 提交边界

```text
feat: add layout component shells
```

---

## 设计核对点

- [ ] 单用户、自托管定位：不引入多用户、RBAC、企业协作概念
- [ ] Hub / Agent 均为单二进制 + supervisor + worker 模型
- [ ] 第一阶段 supervisor 只负责启动、监控、重启
- [ ] 前端工程结构符合功能域组织（features/、pages/、components/）
- [ ] 主题 token 从 prototype/css/variables.css 迁移，保持一致性
- [ ] i18n key 结构从 prototype/shared.js 的 I18N 对象参考
- [ ] 不引入自动更新、文件传输等超前功能
- [ ] 文件传输数据不经过浏览器（M0 不涉及，但架构预留）
