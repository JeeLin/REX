# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.91.0 T2 | 3 | TransferCoordinator: cancel-clobber window re-entry | @oracle | `is_canceled()` 僅於 loop 頂部與預檢查；`if offset >= total { break; }` 直通 `set_status(Verifying)`/`Completed` 之間無同步檢查 — 若取消標記在最後輪詢後、abort 生效前落庫，可能覆蓋 `canceled`→`verifying`/`completed`，任由 `.rex.part` 殘留。預先存在，未由 T2 EOF-guard 調寬。建議：loop 結束前 re-check `is_canceled()`。 |
| v0.91.0 T2 | 3 | TransferCoordinator: 非 Canceled 錯誤路徑未落 `Failed` | @oracle | `Download`/`Upload`/`SourceStat` 錯誤透過 `?` 傳播不落狀態 → 任務卡在 `running`/`verifying` (除 `Delete`/`Verify`/`Rename` 路徑已設 `Failed`)。預先存在；T2 未修。建議：funnel `TransferError` → `set_status(Failed)`（不覆蓋 `canceled`）。 |
