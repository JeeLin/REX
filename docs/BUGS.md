# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| 0.70.2 | 🟢 | 清理 baresip FFI 的 arc_with_non_send_sync clippy 警告 | 步骤4代码精简 | audio_bridge.rs/baresip_ua.rs/capture.rs 构造处缺 allow，前序子任务遗留；CI 默认门禁（exit 0）不受影响 ||
