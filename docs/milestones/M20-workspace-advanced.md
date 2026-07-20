# M20: 工作区高级功能（广播模式 + 深度属性 + Quick Connect 增强）

## Context

M0–M19 完成了基础设施、核心功能和文件管理增强。工作区已具备 Tab 管理、分屏布局、SFTP 抽屉等能力。本里程碑对标 Xshell 的高级工作区特性，增强多 Tab 协同操作和连接配置深度。

本里程碑版本类型：minor（新功能），版本号 0.20.0 → 0.21.0。

## 产品边界

**本阶段做：**
- Tab 广播模式：「发送到全部」开关，输入同步到所有 SSH Tab，状态栏广播指示器
- 深度资源属性对话框：连接/认证/终端/外观/保活/隧道 5 个分类 Tab
- Quick Connect 增强：协议切换自动补全端口、密码字段、连接历史下拉

**本阶段不做：**
- Tab 跨 Pane 拖动（需分屏系统重构，后续里程碑）
- 拖出 Tab 分离到新窗口
- 全局查询 / AI 助手

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Tab 广播模式 | ✅ |
| 2 | 深度资源属性对话框 | ✅ |
| 3 | Quick Connect 增强 | ✅ |

## 子任务详细设计

### 1 Tab 广播模式

**功能目标**

多个 SSH Tab 同时输入相同命令，用于批量执行运维操作。

**文件结构**

修改：
- `packages/rex-console-web/src/pages/WorkspacePage.vue` — Tab 数据增加 broadcast 字段，状态栏增加广播指示器
- `packages/rex-console-web/src/features/terminal/TerminalView.vue` — 接收 broadcast 输入事件

**数据模型**

```typescript
// Tab 接口增加
interface Tab {
  // ...existing fields
  broadcast?: boolean  // 广播模式开关
}
```

**交互设计**

- Tab 右键菜单增加「Broadcast Input」切换项
- 开启广播后，Tab 名称旁显示 📡 图标
- 状态栏显示「Broadcast: ON」指示器（橙色高亮）
- 广播 Tab 的终端输入同时发送到所有同协议（SSH）的 Tab
- 接收广播的 Tab 状态栏显示「Receiving broadcast」灰色指示

**快捷键**
- `Ctrl+Shift+B`：切换当前 Tab 广播模式

**测试标准**

- 打开 3 个 SSH Tab → 在 Tab A 开启广播 → Tab A 输入命令 → Tab B/C 同时执行
- 状态栏正确显示广播状态
- 关闭广播后输入只影响当前 Tab
- type-check + build 通过

**提交信息**

```
feat(workspace): add Tab broadcast mode for synchronized SSH input
```

### 2 深度资源属性对话框

**功能目标**

将现有简单属性对话框升级为 Xshell 风格的深度 per-session 配置。

**文件结构**

修改：
- `packages/rex-console-web/src/features/workspace/ResourceProperties.vue` — 重写为 5 Tab 深度配置

**交互设计**

```
┌─ Properties: My Server ─────────────────────────────┐
│ [Connection] [Auth] [Terminal] [Appearance] [Keepalive] │
├─────────────────────────────────────────────────────┤
│ Connection Tab:                                      │
│   Host: [192.168.1.100]    Port: [22]               │
│   Protocol: [SSH ▾]                                  │
│                                                      │
│ Auth Tab:                                            │
│   Method: (●) Password  ( ) Key File                 │
│   Username: [root]                                   │
│   Password: [••••••]          [Show]                  │
│   Private Key: [/path/to/key]  [Browse]              │
│   Passphrase: [••••••]                               │
│                                                      │
│ Terminal Tab:                                        │
│   Encoding: [UTF-8 ▾]                                │
│   Scrollback: [10000] lines                          │
│   Cursor: (●) Block ( ) Underline ( ) Bar            │
│   Blink: [✓]                                         │
│                                                      │
│ Appearance Tab:                                      │
│   Theme: [Default ▾]                                 │
│   Font Size: [14]                                    │
│   Opacity: [100]%                                    │
│                                                      │
│ Keepalive Tab:                                       │
│   Send keepalive: [✓]                                │
│   Interval: [60] seconds                             │
└─────────────────────────────────────────────────────┘
```

**Tab 内容**

| Tab | 配置项 |
|-----|--------|
| Connection | Host, Port, Protocol |
| Auth | 认证方式（Password/Key File）、用户名、密码、私钥路径、Passphrase |
| Terminal | 编码、Scrollback 行数、光标样式、光标闪烁 |
| Appearance | 主题、字号、透明度 |
| Keepalive | 保活开关、间隔秒数 |

**后端不变**
- 配置仅在前端 Tab 会话中生效，不持久化到后端
- 后续里程碑可增加后端持久化

**测试标准**

- 右键 Tab → Properties → 显示 5 Tab 深度配置对话框
- 切换各 Tab 验证字段正确
- 修改 Terminal 编码 → 切换到对应 Tab 验证生效
- type-check + build 通过

**提交信息**

```
feat(workspace): upgrade resource properties dialog to deep per-session config
```

### 3 Quick Connect 增强

**功能目标**

Quick Connect 栏增加协议感知、密码字段和连接历史。

**文件结构**

修改：
- `packages/rex-console-web/src/features/workspace/QuickConnect.vue` — 增强为协议感知 + 历史

**交互设计**

```
[SSH ▾] [192.168.1.100] [22] [root] [••••••] [Connect ▾]
                                  ↓
                           ┌─ Recent ─────────────┐
                           │ 192.168.1.100 (root)  │
                           │ 10.0.1.5 (admin)      │
                           │ db.internal (root)     │
                           └───────────────────────┘
```

**功能点**

- 协议切换时自动补全默认端口（SSH=22, MySQL=3306, Redis=6379, PostgreSQL=5432）
- 增加密码输入框（type=password，可切换显示）
- Connect 按钮增加下拉箭头：「Connect」/「Connect & Save」（保存为资源）
- 连接历史下拉：最近 10 个连接，点击自动填充
- 历史存储在 localStorage

**测试标准**

- 切换协议 → 端口自动更新
- 输入连接信息 → 点 Connect → 打开对应 Tab
- 点 Connect & Save → 弹出资源创建向导预填
- 连接后历史记录更新
- 下次打开 Quick Connect → 历史下拉可选
- type-check + build 通过

**提交信息**

```
feat(workspace): enhance Quick Connect with protocol-aware port, password, and history
```

## 设计核对点

- [ ] Tab 广播模式可用（输入同步到多个 SSH Tab）
- [ ] 状态栏广播指示器
- [ ] 深度属性对话框 5 Tab 完整
- [ ] Quick Connect 协议自动补全端口
- [ ] Quick Connect 密码字段
- [ ] Quick Connect 连接历史
- [ ] type-check + build 通过

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
