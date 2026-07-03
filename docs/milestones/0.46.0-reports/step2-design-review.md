# 0.46.0 Step 2: Design Review

## Review Date: 2026-07-03

## 1. Product Alignment (vs PRODUCT.md 3.6)

### Toolbar
| Product Spec | Milestone Coverage | Status |
|---|---|---|
| 左侧：连接状态指示 + 资源名称 + 延迟（ms） | 子任务1：延迟测量与工具栏显示 | ✅ Covered |
| 右侧：复制、粘贴、清屏、SFTP、全屏、断开 | 已实现（WorkspaceToolbar），本里程碑不修改 | ✅ Already done |
| 每 5 秒 ping 测量延迟 | 子任务1：测量频率每 5 秒 | ✅ Covered |

### Status Bar
| Product Spec | Milestone Coverage | Status |
|---|---|---|
| 左侧：协议·编码·终端尺寸 | 已实现 | ✅ Already done |
| 右侧：连接方式·操作提示 | 子任务2：状态栏补全 | ✅ Covered |

### Right-Click Menu (10 items in PRODUCT.md)
| Product Spec | Current | Milestone | Status |
|---|---|---|---|
| 复制 | ✅ | — | ✅ |
| 粘贴 | ✅ | — | ✅ |
| 全选 | ✅ | — | ✅ |
| 清屏 | ✅ | — | ✅ |
| 重连 | ✅ | — | ✅ |
| 打开 SFTP 面板 | ✅ | — | ✅ |
| 在新标签中打开 SFTP | ❌ Missing | 子任务2 | ✅ Covered |
| 新建 SSH 连接 | ✅ | — | ✅ |
| 复制连接地址 | ✅ | — | ✅ |
| 断开连接 | ✅ | — | ✅ |

### Mobile Floating Toolbar
| Product Spec | Current | Status |
|---|---|---|
| 方向键（↑↓←→） | ✅ Implemented | ✅ Already done |
| Tab/Enter/^C/^L | ✅ Implemented | ✅ Already done |
| 历史/粘贴/字体缩放/更多 | ✅ Implemented | ✅ Already done |

### i18n
| Product Spec | Milestone Coverage | Status |
|---|---|---|
| 所有用户可见文本支持中英文 | 子任务3：Terminal.vue 硬编码中文修复 | ✅ Covered |

## 2. Product Boundary Check

- ✅ No multi-user/RBAC concepts introduced
- ✅ No product documentation modification
- ✅ No new UI styles (latency colors use CSS variables)
- ✅ No new terminal emulation features
- ✅ Single-user, self-hosted design maintained

## 3. Architecture Consistency

- ✅ Follows existing WebSocket message pattern (ping/pong is standard)
- ✅ Frontend follows existingContextMenu pattern
- ✅ i18n follows existing ws.terminal.* key structure
- ✅ No new Rust crates or frontend packages introduced

## 4. Subtask Granularity

- 子任务1 (延迟测量): 后端 ping/pong + 前端显示 → 1 commit ✅
- 子任务2 (菜单+状态栏): 右键菜单 + 状态栏 → 1 commit ✅
- 子任务3 (i18n): Terminal.vue 修复 → 1 commit ✅

All subtasks are appropriately scoped at 1-2 commits each.

## 5. Design Checkpoints

- [x] 单用户设计：无权限检查
- [x] 自托管：所有功能本地运行
- [x] 深色主题一致性：延迟颜色使用 CSS 变量
- [x] i18n 覆盖：所有新增文本中英文
- [x] 复用现有组件（ContextMenu、useContextMenu composable）
- [x] 不引入新概念（延迟测量是标准 WebSocket ping/pong）

## Issues Found

### Minor Issues (corrected)
1. **Context 中右键菜单数量不准确**：原文写"PRODUCT.md 定义了 12 项"，实际为 10 项。已修正为正确数字。

## Conclusion

里程碑文档与 PRODUCT.md 3.6 的规格完全对齐。所有产品差距（延迟显示、右键菜单、状态栏、i18n）均在子任务中覆盖。子任务粒度合理，每个子任务对应 1 个 commit。产品边界正确，未引入超出范围的功能。

**结论: ✅ 通过**
