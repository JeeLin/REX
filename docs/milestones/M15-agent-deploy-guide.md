# M15: Agent 部署指南改进

## Context

M12 增强了 Agent 管理页面（配置弹窗、日志查看器、令牌重置），但部署指南（DeployGuide）仍然是简单的代码块展示。用户要求匹配原型 `prototype/agents.html` 的详细分步说明格式。

## 产品边界

**做什么：**
- 部署指南改为分步编号说明（1. 下载 Agent → 2. 获取注册令牌 → 3. 启动 Agent → 4. 确认连接）
- 每个平台提供下载按钮（linux-amd64/arm64/armv7l, darwin-arm64/amd64, windows-amd64）
- 令牌展示框 + 复制按钮
- 代码块 + 复制按钮
- 保持 4 个 tab（二进制文件、Docker、Docker Compose、配置文件）

**不做什么：**
- 后端 Agent 下载 API（已在 M12 实现）
- Agent 令牌管理后端（已在 M2 实现）
- 移动端适配

---

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 15.1 | 部署指南组件重构 | 前端 | ✅ |

---

## 子任务 15.1：部署指南组件重构

### 功能目标

重构 `DeployGuide.vue`，从简单代码块展示改为原型风格的分步说明格式。

### 修改文件

```text
packages/rex-console-web/src/
├── features/agents/DeployGuide.vue    修改：重构组件
├── i18n/zh.ts, en.ts                  修改：添加部署指南 i18n
```

### 交互设计

参考原型 `prototype/agents.html` 第 415-565 行：

**组件结构：**

```text
DeployGuide
├── 标题区
│   ├── 标题：「部署新 Agent」
│   └── 副标题：「在需要管理的内网环境中部署 Agent...」
├── Tab 栏（二进制文件 / Docker / Docker Compose / 配置文件）
└── Tab 内容区
    ├── 步骤 1：下载 Agent
    │   ├── 步骤编号（圆形背景）
    │   ├── 步骤标题
    │   ├── 步骤说明
    │   ├── 平台下载按钮网格（6 个按钮）
    │   └── 代码块 + 复制按钮（curl 命令）
    ├── 步骤 2：获取注册令牌
    │   ├── 步骤说明
    │   └── 令牌展示框 + 复制按钮
    ├── 步骤 3：启动 Agent
    │   ├── 步骤说明
    │   └── 代码块 + 复制按钮
    └── 步骤 4：确认连接
        └── 步骤说明
```

**各 Tab 步骤内容：**

**二进制文件 Tab：**

| 步骤 | 标题 | 内容 |
|------|------|------|
| 1 | 下载 Agent | 6 个平台下载按钮 + curl 命令代码块 |
| 2 | 获取注册令牌 | 说明文字 + 令牌展示框（从环境详情获取） |
| 3 | 启动 Agent | `./rex-agent --server ... --token ... --name ...` 代码块 |
| 4 | 确认连接 | 回到 Hub 确认 Agent 在线 |

**Docker Tab：**

| 步骤 | 标题 | 内容 |
|------|------|------|
| 1 | 拉取镜像 | `docker pull rexhub/agent:latest` 代码块 |
| 2 | 启动容器 | `docker run ...` 代码块（含环境变量） |
| 3 | 确认连接 | 回到 Hub 确认 Agent 在线 |

**Docker Compose Tab：**

| 步骤 | 标题 | 内容 |
|------|------|------|
| 1 | 创建配置 | `docker-compose.yml` 代码块 |
| 2 | 启动服务 | `docker compose up -d` 代码块 |
| 3 | 确认连接 | 回到 Hub 确认 Agent 在线 |

**配置文件 Tab：**

| 步骤 | 标题 | 内容 |
|------|------|------|
| 1 | 创建配置文件 | `agent.yaml` 代码块 |
| 2 | 启动 Agent | `./rex-agent --config agent.yaml` 代码块 |
| 3 | 确认连接 | 回到 Hub 确认 Agent 在线 |

**下载按钮网格：**

```text
⬇ linux-amd64  ⬇ linux-arm64  ⬇ linux-armv7l
⬇ darwin-arm64 ⬇ darwin-amd64 ⬇ windows-amd64
```

每个按钮链接到 `/api/agent/download?os={os}&arch={arch}`。

**令牌展示框：**

```text
┌──────────────────────────────────────────────────┐
│ 令牌  │ rex_env_...（截断显示）         │ 复制  │
└──────────────────────────────────────────────────┘
```

- 令牌从 props 传入（`agentToken`）
- 点击复制后按钮文字变为「已复制」，2 秒后恢复

**代码块复制按钮：**

每个代码块右上角有「复制」按钮，点击后复制代码内容，按钮文字变为「已复制」。

### 接口设计

```ts
// DeployGuide.vue props
interface Props {
  agentToken?: string   // Agent 注册令牌（从环境详情传入）
}
```

### 实现要点

1. 步骤编号使用圆形背景 + 数字
2. 下载按钮使用 `<a>` 标签链接到下载 API
3. 令牌框使用 monospace 字体，支持文本截断
4. 复制按钮使用 `navigator.clipboard.writeText`
5. 所有用户可见文字使用 i18n
6. 代码块保留 `white-space: pre` 格式

### 测试标准

- 4 个 tab 正确切换
- 步骤编号 1-4 正确显示
- 下载按钮链接到正确的 API 路径
- 令牌框正确显示和复制
- 代码块复制按钮功能正常
- 与原型 agents.html 第 415-565 行视觉一致

### 提交信息

```
feat: redesign agent deploy guide with step-by-step instructions
```

---

## 设计核对点

- [ ] 部署指南与原型 agents.html 一致
- [ ] 下载按钮链接到正确的 API
- [ ] 令牌框和复制功能正常
- [ ] 代码块复制功能正常
- [ ] i18n 覆盖所有新增文字

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
