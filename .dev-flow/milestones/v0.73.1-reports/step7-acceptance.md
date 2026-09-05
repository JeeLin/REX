# 功能验收：v0.73.1 Agent SSH 连接修复

## 验收原则

- 从 git diff (milestone-v0.73.1-start) 出发，逐文件审查
- 不信任 ✅/[x] 流程标记
- 不引用步骤 3 的提交信息
- 每个子任务和 bug 独立验证

## 变更概览

- **变更文件**：2 个核心文件（+1 milestone doc, +1 report）
- **基准 ref**：milestone-v0.73.1-start
- **验收时间**：2026-07-05

## 子任务验收

| # | 子任务 | 结论 | 证据 | 说明 |
|---|--------|------|------|------|
| 1 | 定位问题根因 | ✅ | `agent_ssh.rs` diff — Mutex 死锁分析正确 | 修复代码精准针对 `Arc<Mutex<SshSession>>` 死锁：out_task 持锁 recv() 与 in_task 持锁 send_data() 竞争。split() 拆分方案根治问题 |
| 2 | 修复 SSH 连接 | ✅ | `agent_ssh.rs` + `rex-ssh/src/lib.rs` | split() 拆分会话为写半区+事件接收器；out_task 直接拥有 events 无需锁；in_task 通过 Arc<write_half> 共享无竞争；send→data_bytes / resize→window_change API 对齐 |
| 3 | 回归测试 + 质量门禁 | ✅ | step6-test.md | cargo fmt/clippy/test + bun type-check/lint/build 全部通过 |

## Bug 修复验收

| # | 优先级 | 标题 | 结论 | 证据 | 说明 |
|---|--------|------|------|------|------|
| 1 | 🔴 | Agent SSH 连接后无法输入命令，后续连接全部失败 | ✅ | `agent_ssh.rs` Mutex→split 重构 | 根因是 Arc<Mutex> 死锁导致命令输入阻塞；split() 拆分后 out_task/in_task 无锁竞争，首次连接可正常交互，连接关闭后资源释放，后续连接正常建立 |

## 未覆盖检查

- ✅ 无遗漏子任务（3/3 全部验证）
- ✅ 无遗漏 bug 修复（1/1 已验证）
- ✅ 无范围外变更（仅修改 Agent SSH 相关 2 文件，Hub 直连路径未受影响）

## 汇总

- **子任务通过**：3/3
- **Bug 修复通过**：1/1
- **结论**：✅ 验收通过
