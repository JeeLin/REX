# 步骤4：代码精简（0.70.3）

- **执行范围**：`git diff --name-only milestone-0.70.3-start` 命中的本里程碑改动，外加全仓库 `cargo clippy --workspace --all-targets` 复检。
- **日期**：2026-08-18

## 发现与处置

| # | 位置 | 问题 | 级别 | 处置 |
|---|------|------|------|------|
| S1 | `crates/rex-sip/src/capture.rs:80,89` | 两层 `if !x.is_null() { if sa_ntop(...) > 0 {...} }` 可折叠为单层判断 | 🟢 风格 | 折叠为 `if !x.is_null() && sa_ntop(...) > 0`，行为不变 |
| S2 | `crates/rex-hub/src/sip_ws.rs:~1299` | `decode_video_frame(&rest[..])` 冗余切片 | 🟢 风格 | 改为 `decode_video_frame(rest)` |
| S3 | `crates/rex-hub/src/middleware.rs:233` | `#[cfg(test)] mod tests` 中 `use super::*;` 未使用 | 🟢 风格 | 删除未使用导入 |
| S4 | `crates/rex-hub/tests/api_integration.rs:213` | `len() >= 1` 长度比较 | 🟢 风格 | 改为 `!is_empty()` |
| B1 | `crates/rex-redis/src/redis_codec.rs` `try_pickle` | 🟡 bug：单字节 opcode（0x62/0x28/0x63）即判 Pickle，正常文本（`b:1;`、`compress me`）误报 | 🟡 必须修复 | 加结构性约束：高字节 opcode 直接判；`0x43/0x42/0x62` 须长度字段落在缓冲内；`0x63 GLOBAL` 须后随 `module\nclass\n`；移除 `0x28 MARK` 判据。补 3 条回归测试 |

## 结论

- 4 处 clippy 风格警告（S1–S4）均为行为无关的等价改写，已就地修复（属本里程碑内、非审查阶段新发现，与 #6 一并提交）。
- 1 处 🟡 检测质量 bug（B1）已修复并补回归测试，`cargo test -p rex-redis` 21 项全过。
- 剩余的 `arc_with_non_send_sync` 3 处警告（audio_bridge.rs / baresip_ua.rs / capture.rs）为缺陷池 🟢，归属子任务 #8，不在本步打回范围。

**门禁**：无 🔴 发现；🟡（B1）已修复达标，不阻断。仅 🟢 风格项已入本提交，无新增缺陷池条目。

## 步骤4 报告结论

✅ 通过（无 🔴/🟡 未修复项）。
