# 0.86.0 代码审查报告

## 审查范围

所有里程碑变更（`7165ff2` ~ `c8d864b`），28 个文件 +1595 -124 行。

## 🔴 必须修复

**无。**

## 🟡 应该修复

### 1. `task.rs:165-171` — `set_max_concurrent` 存在理论竞态

`running_count()` 读取当前运行数 → 新建 `Semaphore::new(permits)` → 赋值替换旧信号量。

竞态窗口：两个线程同时调用 `set_max_concurrent`，或调用与 `acquire_permit` 并行时，新信号量的许可数可能略多于实际需求。

**实际影响**：极低。`set_max_concurrent` 仅由设置 API 触发，单用户自托管场景下几乎不可能并发调用；且偏差最多 1 个槽位，不会造成数据损坏。

**结论**：🟡 标记，不阻塞发布。若未来有并发调用需求，可引入 `ArcSwap` 或合并信号量更新与 max_concurrent 为单次原子操作。

### 2. `task.rs:196-201` — `acquire_permit` 的 `expect("semaphore closed unexpectedly")`

`Semaphore` 在 `set_max_concurrent` 中被替换（`Arc::new`），旧信号量的引用计数归零时 drop。若有任务正在 `acquire` 已 drop 的信号量，`acquire_owned` 返回 `Err`，`expect` 会 panic。

**实际影响**：仅在 `set_max_concurrent` 与 `acquire_permit` 高度并发时可能触发（极罕见）。panic 会被 supervisor 进程捕获并重启。

**结论**：🟡 标记，不阻塞发布。改进方案：将 `expect` 改为 `match` + 回退到新信号量重试。

## 🟢 可选改进

### 3. `agent_download.rs:65-84` — 本地路径搜索 3 分支可合并

`flat_path`、`nested_path`、`nested_path.exe` 三路 `if-else if`，Windows `.exe` 分支重复了路径拼接。可简化为候选列表循环查找。

**价值**：代码更简洁，新增布局仅需追加一行候选。

### 4. `BatchImportDialog.vue:219-236` — FileReader 未处理 `onerror`

`processFile` 使用 `FileReader.readAsText` 但未注册 `onerror` 回调，文件读取失败时 `parseError` 保持空字符串。

**价值**：改进用户反馈。

### 5. `useTabs.ts:170` — `reorderTab` 直接修改数组原地

`reorderTab` 中 `tabs.splice(fromIdx, 1); tabs.splice(toIdx, 0, tab)` 直接 mutate 数组。虽在 Vue 3 中因 `ref` 仍可触发响应式更新，但 mutate + 重新赋值是冗余的。此模式与文件中已有代码一致，不做改动。

## 审查结论

- 🔴 **0 项**（无阻塞）
- 🟡 **2 项**（理论竞态 + expect panic，均为极端路径，不影响单用户场景）
- 🟢 **3 项**（可选改进）

✅ **通过，无 🔴 阻塞项。**
