# M7 设计核对报告

## 审查范围

里程碑文档 `docs/milestones/M7-auto-update.md` vs 产品文档 + 开发文档

## 核对维度

### 1. 是否符合产品定位
✅ 通过。单用户自托管，更新仅限 Hub/Agent 自身。

### 2. 架构一致性
✅ 通过。复用 M0 的 supervisor + worker 模型，退出码语义与 DEVELOPMENT.md §4 一致。

### 3. 文件传输不经过浏览器
✅ 通过。M7 不涉及文件传输。

### 4. 不引入超前概念
✅ 通过。明确排除 Windows 副本、Docker 内替换、签名验证。

### 5. 不跳阶段
✅ 通过。严格限制在替换+回滚，不引入 CI/CD、自动发布等。

### 6. 与 DEVELOPMENT.md 里程碑 8 一致性

| DEVELOPMENT.md 任务 | M7 子任务 | 一致性 |
|---------------------|-----------|--------|
| update-state.json schema | 7.1 | ✅ |
| 状态原子写入 | 7.1 | ✅ |
| staging 下载目录 | 7.2 | ✅ |
| rollback 备份目录 | 7.2 | ✅ |
| SHA256 校验 | 7.3 | ✅ |
| Unix/Linux/macOS 原子替换 | 7.4 | ✅ |
| worker 退出码 10/11/12 | 7.4 | ✅ |
| REX_UPDATE_PENDING | 7.4 | ✅ |
| Hub 新 worker 健康检查 | 7.5 | ✅ |
| Agent 新 worker 健康检查 | 7.5 | ✅ |
| 连续 3 次失败回滚 | 7.5 | ✅ |
| Windows supervisor 副本 | — | 明确排除 |

## 结论

✅ **通过** — 里程碑文档与产品文档、开发文档一致。
