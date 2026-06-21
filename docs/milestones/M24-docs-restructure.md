# M24: 开发文档重构

## Context

M0-M23 已完成所有核心产品功能。当前 `docs/DEVELOPMENT.md` 是一个约 5190 行的巨型文件，包含技术栈、crate 结构、进程模型、退出码、更新状态、文件传输、API 端点、配置、Docker、前端工程、工作区、SSH 终端、SQL、文件传输、Agent、审计日志、更新实现、数据模型、API 设计、连接通道、Docker 构建、所有里程碑子任务（0-9）等全部内容。

问题：
- 单文件过长，难以定位和维护
- 里程碑子任务详细设计（M0-M8）已实现完毕，保留意义不大
- 前后端分离在文档中未体现
- 架构决策、数据模型、API 设计等参考信息与里程碑任务混杂

## 产品边界

**做什么：**
- 将 DEVELOPMENT.md 拆分为按职责组织的子目录
- 保留架构决策、数据模型、API 设计等长期参考内容
- 将已实现的里程碑子任务详细设计精简为里程碑摘要
- 更新 CLAUDE.md 指向新结构
- 同时包含后端接口和前端入口的里程碑文档

**不做什么：**
- 不修改 PRODUCT.md
- 不修改任何代码
- 不删除里程碑文档（docs/milestones/）

---

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 24.1 | 创建 docs/architecture/ 目录，提取架构文档 | ✅ |
| 24.2 | 创建 docs/reference/ 目录，提取参考文档 | ✅ |
| 24.3 | 重写 DEVELOPMENT.md 为精简索引 | ✅ |
| 24.4 | 更新 CLAUDE.md 指向新结构 | ✅ |

---

## 子任务详细设计

### 24.1 创建 docs/architecture/ 目录

**功能目标：**
将 DEVELOPMENT.md 中的架构决策提取为独立文档，便于查阅和维护。

**创建文件：**
```text
docs/architecture/
├── process-model.md          进程模型（supervisor + worker）
├── update-mechanism.md       更新机制（退出码、状态机、回滚）
├── file-transfer.md          文件传输架构（FileConnector trait、传输路径）
├── connection-channels.md    连接通道（直连 / Agent 代理）
└── docker.md                 Docker 构建与部署
```

**内容来源：** DEVELOPMENT.md 中的 §3（进程模型）、§4（退出码语义）、§5（更新状态文件）、§6（文件传输）、§10（Docker 信号处理）、§11（Windows 差异）、§25（连接通道）、§26（Docker 构建）。

**提取原则：**
- 保留原始内容，不修改技术细节
- 每个文件独立完整，可单独阅读
- 保留代码示例和流程图

---

### 24.2 创建 docs/reference/ 目录

**功能目标：**
将 DEVELOPMENT.md 中的参考信息提取为独立文档，供开发时查阅。

**创建文件：**
```text
docs/reference/
├── data-models.md            数据模型（SQL 表结构、ID 格式）
├── api-design.md             API 设计规范（认证、错误格式、分页、WebSocket）
├── frontend-architecture.md  前端工程结构与组件规范
└── config-conventions.md     配置与目录约定
```

**内容来源：** DEVELOPMENT.md 中的 §23（数据模型）、§24（API 设计规范）、§12（前端工程结构）、§20（配置与目录约定）、§22（后端工程结构）。

---

### 24.3 重写 DEVELOPMENT.md

**功能目标：**
将 DEVELOPMENT.md 重写为精简索引，指向各子文档，不再包含详细实现内容。

**新内容结构：**
```text
# REX Hub — 开发文档

## 1. 技术栈
（保留 §1 表格，~10 行）

## 2. Rust crate 结构
（保留 §2 结构概览，~20 行）

## 3. 里程碑总览
（新增表格，列出 M0-M23 状态和摘要）

## 4. 架构文档
链接到 docs/architecture/

## 5. 参考文档
链接到 docs/reference/

## 6. 里程碑详细文档
链接到 docs/milestones/
```

**目标行数：** ~200 行（原 5190 行）

---

### 24.4 更新 CLAUDE.md

**功能目标：**
更新 CLAUDE.md 中的开发文档引用和仓库结构说明。

**修改内容：**
1. 仓库结构部分：更新 docs/ 目录说明
2. 开发文档引用：指向新索引 + 子目录
3. 里程碑文档规则：标注每个里程碑同时包含后端和前端

---

## 设计核对点

- [ ] 新文档结构与 PRODUCT.md 一致
- [ ] 架构文档保留所有关键决策
- [ ] 数据模型完整保留
- [ ] API 设计规范完整保留
- [ ] CLAUDE.md 更新后指向正确路径
- [ ] 不引入新概念或修改产品语义

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
