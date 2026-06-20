# M9 代码审查报告

## 变更范围

- `.github/workflows/ci.yml` — Hub 多平台构建矩阵 + Release 包含 Hub 二进制和 SHA256SUMS
- `docs/RELEASE-TEMPLATE.md` — 新增 Release 说明模板
- `docs/UPGRADE.md` — 新增升级说明文档

## 审查发现

### 🔴 必须修复
无

### 🟡 应该修复
无

### 🟢 可选改进
- build-hub 和 build-agent 矩阵配置高度相似，可考虑用 YAML anchor 或 reusable workflow 去重（但 GitHub Actions 对 YAML anchor 支持有限）

## 架构一致性

- ✅ Hub 依赖 `build-frontend` 确保前端静态资源已构建
- ✅ Release 依赖 `build-agent` + `build-hub` 确保所有二进制就绪
- ✅ Docker Hub 镜像仅下载 linux-amd64 Hub 二进制（Docker 镜像只支持 amd64）
- ✅ SHA256SUMS 覆盖所有平台 Hub + Agent zip

## 安全性

- ✅ SHA256SUMS 校验与 M6/M7 更新机制一致
- ✅ Release 权限 `contents: write` 仅在 release job 中使用

## 结论

✅ **审查通过** — 无 🔴 必须修复项。
