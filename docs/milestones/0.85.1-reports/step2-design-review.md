# Step 2: 设计核对报告 — 0.85.1 CI 构建优化

## 审查维度

### 1. 产品边界合理性
- ✅ CI 构建优化是纯内部工程改进，不涉及产品功能变更
- ✅ 不会引入多用户、RBAC 等违反项目约束的概念
- ✅ 范围聚焦：仅优化 GitHub Actions CI 流水线

### 2. 子任务拆分粒度
- ✅ 5 个子任务，每个聚焦一个独立优化方向
- ✅ 子任务间依赖关系清晰：1(Docker cache)、2(rust-cache)、3(并行度)、4(交叉编译)、5(验证) 可并行执行 1-4，5 为收尾
- ✅ 每个子任务有明确的提交信息，粒度合理

### 3. 技术方案可行性
- ✅ Docker BuildKit GHA caching 是成熟方案，GitHub Actions 原生支持
- ✅ Swatinem/rust-cache 的 shared-key 和 save-if 是已验证的优化手段
- ✅ build-hub 去掉 build-frontend 依赖：检查确认 build-hub 只编译 Rust 二进制，不使用 frontend dist（frontend dist 仅在 docker-hub 中打包），方案正确
- ✅ cargo-zigbuild 是 Rust 社区广泛使用的交叉编译工具

### 4. 风险评估
- 🟡 cargo-zigbuild 对 rex-hub 的特定依赖（如 ring、openssl）可能有兼容性问题 → 子任务 4 已包含回退方案（保持 cross + 优化 Docker image 缓存）
- 🟡 rust-cache shared-key 可能导致不同 target 缓存互相干扰 → 已按 target 分配独立 key
- 🟢 Docker cache-from/to 在 tag push 以外的场景（如 PR）不触发，符合预期

### 5. 与现有代码/架构一致性
- ✅ 仅修改 `.github/workflows/ci.yml`，不涉及 Rust 或前端代码
- ✅ 与 AGENTS.md 中 CI 优化方向一致（Docker layer caching、构建矩阵并行、增量编译复用、交叉编译优化）

## 结论

✅ 通过。里程碑文档设计合理，子任务拆分清晰，技术方案可行，风险可控。

### 建议
- 子任务 4（cargo-zigbuild）实施时应先验证 rex 的依赖链（特别是 ring crate）与 zigbuild 的兼容性
