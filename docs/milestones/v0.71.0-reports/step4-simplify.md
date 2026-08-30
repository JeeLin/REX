# Step 4: Code Simplify — v0.71.0 Full UI/UX Redesign

## 检查范围
14 files changed since milestone-v0.71.0-start

## 检查结果

### 🟢 可选改进（不阻塞）

| # | 位置 | 问题 | 说明 |
|---|------|------|------|
| 1 | LoginPage.vue, SetupPage.vue | 装饰性 rgba 渐变 | 登录页的 radial-gradient 背景效果使用了 rgba 值，难以 token 化，属于视觉装饰 |
| 2 | DashboardPage.vue:307 | #2dd4bf teal 色缺 token | 新增的 teal 强调色（`--teal: #2DD4BF`）未在 tokens.css 中定义 |
| 3 | AppLayout.vue:555 | 头像渐变硬编码 | `#3b82f6, #1d4ed8` 蓝色渐变，可用 token 但当前唯一 |

### 无 🔴/🟡 发现

无死代码、无重复逻辑、无过长函数、无功能风险。

## 结论
无 🔴/🟡 发现，🟢 发现 3 处（装饰性渐变 + 缺 teal token），不阻塞流程。
