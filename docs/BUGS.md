# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.91.0 T2 | 3 | TransferCoordinator: cancel-clobber window re-entry | @oracle | `is_canceled()` 仅于 loop 顶部与预检查；`if offset >= total { break; }` 直通 `set_status(Verifying)`/`Completed` 之间无同步检查 — 若取消标记在最后轮询后、abort 生效前落库，可能覆盖 `canceled`→`verifying`/`completed`，任由 `.rex.part` 残留。预先存在，未由 T2 EOF-guard 调宽。建议：loop 结束前 re-check `is_canceled()`。 |
| v0.91.0 T2 | 3 | TransferCoordinator: 非 Canceled 错误路径未落 `Failed` | @oracle | `Download`/`Upload`/`SourceStat` 错误透过 `?` 传播不落状态 → 任务卡在 `running`/`verifying` (除 `Delete`/`Verify`/`Rename` 路径已设 `Failed`)。预先存在；T2 未修。建议：funnel `TransferError` → `set_status(Failed)`（不覆盖 `canceled`）。 |
| v0.91.0 T4 | 3 | S3Connector::upload offset>0 多部件续传不对齐 — 仅影响已弃用 browser /upload（已下线） | @oracle | `upload(offset>0)` 建立新 `upload_id`、parts 从 `offset/5MiB+1` 开始，但 `chunks(part_size)` 从 `data[0]` 迭代、offset 与 5MiB 不对齐（prod 传 offset-relative data 但 1MiB chunk）。production `transfer_coordinator` 路径在 `size-verify guard:278` 捕获（非 regressing）；唯一危险路径是已弃用的 browser `/upload` handler（`file_api.rs:751/764`）。正确续传应使用 `resume_multipart_upload(upload_id)`（`file_transfer.rs:399` / `s3 lib.rs:537`）。T6 将 drop/gate `/upload` 的 offset 栏位。 |
