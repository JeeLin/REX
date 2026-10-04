# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.91.0 T2 | 3 | TransferCoordinator: cancel-clobber window re-entry | @oracle | `is_canceled()` 僅於 loop 頂部與預檢查；`if offset >= total { break; }` 直通 `set_status(Verifying)`/`Completed` 之間無同步檢查 — 若取消標記在最後輪詢後、abort 生效前落庫，可能覆蓋 `canceled`→`verifying`/`completed`，任由 `.rex.part` 殘留。預先存在，未由 T2 EOF-guard 調寬。建議：loop 結束前 re-check `is_canceled()`。 |
| v0.91.0 T2 | 3 | TransferCoordinator: 非 Canceled 錯誤路徑未落 `Failed` | @oracle | `Download`/`Upload`/`SourceStat` 錯誤透過 `?` 傳播不落狀態 → 任務卡在 `running`/`verifying` (除 `Delete`/`Verify`/`Rename` 路徑已設 `Failed`)。預先存在；T2 未修。建議：funnel `TransferError` → `set_status(Failed)`（不覆蓋 `canceled`）。 |
| v0.91.0 T4 | 3 | S3Connector::upload offset>0 多部件續傳不對齊 — 僅影響已棄用 browser /upload（已ゲート） | @oracle | `upload(offset>0)` 建立新 `upload_id`、parts 從 `offset/5MiB+1` 開始，但 `chunks(part_size)` 從 `data[0]` 迭代、offset 與 5MiB 不對齊（prod 傳 offset-relative data 但 1MiB chunk）。production `transfer_coordinator` 路徑在 `size-verify guard:278` 捕獲（非 regressing）；唯一危險路徑是已棄用的 browser `/upload` handler（`file_api.rs:751/764`）。正確續傳應使用 `resume_multipart_upload(upload_id)`（`file_transfer.rs:399` / `s3 lib.rs:537`）。T6 將 drop/gate `/upload` 的 offset 欄位。 |
