# 0.5.0 步骤2：设计核对报告

## 审查范围

里程碑文档 `0.5.0-docker-compose.md` vs 产品文档 `PRODUCT.md`

## 逐项检查

### 产品定位

| 检查项 | 结果 |
|--------|------|
| 单用户、自托管 | ✅ docker-compose 不引入多用户概念 |
| 不引入 RBAC | ✅ |
| 不引入新依赖 | ✅ curl 是 docker-compose healthcheck 的运行时依赖，不是项目代码依赖 |

### 架构一致性

| 检查项 | 结果 |
|--------|------|
| 单二进制 + supervisor + worker | ✅ docker-compose 不改变进程模型 |
| 文件传输不经过浏览器 | ✅ docker-compose 只是部署配置 |
| Hub/Agent 版本一致 | ✅ 两个 compose 文件使用同一 tag |

### Docker 配置

| 检查项 | 结果 |
|--------|------|
| 镜像地址与 CI 一致 | ✅ ghcr.io/${GITHUB_REPO_OWNER}/rex-hub |
| 数据卷路径与 Dockerfile DATA_DIR 一致 | ✅ /app/data |
| 健康检查端点正确 | ✅ /healthz（M1 实现） |
| healthcheck 可用性 | ✅ 需在 Dockerfile.hub 加装 curl（已在文档中标注） |
| Agent Docker.sock 挂载 | ✅ 与 PRODUCT.md 一致 |
| Agent 数据卷持久化 | ✅ agent.json 保留在 /app/data |

### CI 调整

| 检查项 | 结果 |
|--------|------|
| Release 使用 CHANGELOG | ✅ 替代 generate_release_notes |
| Agent Docker README | ✅ docs/agent-readme.md 用于 Docker Hub 页面 |

### 子任务粒度

| 检查项 | 结果 |
|--------|------|
| 每个子任务对应 1 个 commit | ✅ |
| 子任务编号与详细设计一致 | ✅（1-5） |
| 不跳阶段实现 | ✅ |

## 小问题修正

- docker-compose hub healthcheck 需要 curl，已补充 Dockerfile.hub 加装 curl 的说明

## 结论

✅ 设计核对通过。
