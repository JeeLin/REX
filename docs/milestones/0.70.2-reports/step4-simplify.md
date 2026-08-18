# 0.70.2 步骤4：代码精简（simplify，重跑）

背景：首次步骤4 发现 🟡（视频桥 `rx` 队列死代码），已打回开发阶段修复（移除 `rx`/`pop_rx`/`rx_len` 及 `vidisp_disp` 中的 push_back，仅保留 `on_video` 同步下行路径）。本次为重跑后的二次精简检查。

检查范围：`git diff --name-only milestone-0.70.2-start`（全里程碑，子任务 #1–#6）。

## 发现

- 🔴：无
- 🟡：无
- 🟢：1 项（前序子任务遗留，非本里程碑引入，不阻断）
  - baresip FFI `arc_with_non_send_sync` clippy 警告（`audio_bridge.rs`/`baresip_ua.rs`/`capture.rs` 构造处缺 allow），已在首次步骤4 记入缺陷池 `docs/BUGS.md`，下版本统一清理。CI 默认门禁（exit 0）不受影响。

## 走向

视频桥死代码已修复并验证（`cargo test -p rex-sip` 14 passed，`cargo clippy -p rex-sip` 无 `video_bridge` 警告，仅余前序遗留 🟢）。无 🔴/🟡，仅 🟢 入缺陷池，不阻断。

## 门禁结论

精简无 🔴/🟡 发现（🟢 已入缺陷池不阻断）→ 勾选步骤4，继续步骤5。
