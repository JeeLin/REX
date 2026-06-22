# 步骤2：设计核对报告

## 审查范围

里程碑文档 `0.11.0-s3-minio-support.md` vs 产品文档 `PRODUCT.md`。

## 审查框架维度

### 1. 产品边界一致性

| 边界 | PRODUCT.md | 里程碑文档 | 一致 |
|------|-----------|-----------|------|
| 协议定义 | §2.3 S3/MinIO：☁ 图标、#E8912D 主色、对象存储 | S3Config 含 endpoint/access_key/secret_key/region/bucket/force_path_style | ✅ |
| 单用户 | 产品属性：单用户 | 无多用户/RBAC 概念 | ✅ |
| 自托管 | 产品属性：自托管 | 无 SaaS 概念 | ✅ |
| 文件不经过浏览器 | §8 文件传输架构：数据在 Hub/Agent/远端间传输 | stub 模式，upload/download 通过 WebSocket，前端只发 base64 命令 | ✅ |
| 跨连接文件传输 | §3.8：S3 参与跨连接传输 | 产品边界提及，但未在子任务中实现（合理拆分） | ✅ |
| 共享 crate | §8：`rex-s3` 在共享 crate 列表中 | 新增 `rex-s3` crate | ✅ |

### 2. 架构一致性

| 维度 | 现有模式 | 里程碑设计 | 一致 |
|------|---------|-----------|------|
| crate 组织 | 独立 protocol crate（rex-redis/rex-docker/rex-sqlite） | 独立 `rex-s3` crate | ✅ |
| stub 模式 | connector trait + stub impl，实际连接通过 Agent | S3Connector trait + S3ConnectorImpl stub | ✅ |
| WebSocket 协议 | command/response/error/ping/pong/connected/disconnected | 同 | ✅ |
| 路由模式 | `/ws/{protocol}/:resource_id` | `/ws/s3/:resource_id` | ✅ |
| 依赖规则 | workspace = true，根 Cargo.toml 声明版本 | 计划中使用 workspace = true | ✅ |
| 前端功能域 | `features/{protocol}/` + `workspace/panels/` | `features/s3/` + `WorkspaceS3.vue` | ✅ |

### 3. 接口设计审查

| 接口 | 设计合理性 | 备注 |
|------|-----------|------|
| S3Config | ✅ endpoint + 凭据 + bucket + force_path_style 覆盖 S3/MinIO 场景 | |
| S3Connector trait | ✅ 8 个方法覆盖 connect/list/get/upload/download/delete/close | |
| ObjectInfo | ✅ key/size/last_modified/etag/content_type/storage_class/is_dir | |
| BucketInfo | ✅ name + creation_date | |
| WebSocket actions | ✅ buckets/objects/info/upload/download/delete | |
| 前端 useS3Session | ✅ 返回类型与 WebSocket 协议一致 | |

### 4. 依赖选型审查

| 依赖 | 用途 | workspace 状态 | 备注 |
|------|------|---------------|------|
| reqwest | S3 REST API 客户端 | ✅ 已有 | |
| sha2 | AWS Sig V4 签名 | ✅ 已有 | |
| hmac | AWS Sig V4 HMAC | ❌ 需新增 | 已修正：里程碑文档已标注需新增到 workspace.dependencies |
| time | 时间格式化（Sig V4） | ✅ 已有 | 已修正：用 `time` 替代 `chrono`，避免引入新依赖 |

### 5. 子任务拆分审查

| 子任务 | 粒度 | 独立提交 | 合理性 |
|--------|------|---------|--------|
| 1. rex-s3 crate | 1 commit | `feat: add rex-s3 crate with S3Connector trait` | ✅ 与 rex-redis/rex-sqlite 粒度一致 |
| 2. Hub S3 WebSocket | 1 commit | `feat: add S3 WebSocket session management in Hub` | ✅ 与现有模式一致 |
| 3. 前端 S3 控制台 + 资源表单 + 面板 | 1 commit | `feat: add S3 console, resource form, and workspace panel integration` | ✅ 前端子任务合并合理 |

### 6. 交互设计审查

| 交互点 | 设计合理性 | 与现有模式一致 |
|--------|-----------|---------------|
| S3 控制台布局 | ✅ Bucket 列表 → Object 浏览，面包屑导航 | 类似文件管理交互 |
| 资源创建向导 | ✅ S3 特有字段（endpoint/key/secret/bucket/region/path_style） | 与 MySQL/Redis/SQLite 向导模式一致 |
| 工作空间面板映射 | ✅ `s3: 's3'` 映射 | 与 Redis/Docker/SQLite 一致 |
| i18n | ✅ zh/en 基础翻译已存在 | 补充详细翻译 |

### 7. 设计核对点检查

| 检查项 | 结论 |
|--------|------|
| S3 crate 是否遵循现有 protocol crate 的 stub 模式 | ✅ 与 rex-redis/rex-docker/rex-sqlite 一致 |
| S3 连接模型是否正确（endpoint + 凭据 + bucket） | ✅ 覆盖 S3 和 MinIO 两种场景 |
| S3 控制台交互是否直观 | ✅ Bucket 列表 → Object 浏览 → 上传/下载 |
| WebSocket 消息协议是否清晰 | ✅ 与 Redis/Docker/SQLite 协议一致 |
| S3 上传/下载是否通过 WebSocket 通道 | ✅ 文件数据不经过浏览器中转 |
| Path style 选项是否正确支持 MinIO | ✅ force_path_style 默认开启 |
| 是否保持单用户、自托管的产品定位 | ✅ 无多用户/RBAC/SaaS 概念 |
| S3 是否正确参与跨连接文件传输 | ⚠️ 未在子任务中实现（合理拆分，跨连接传输由 rex-transfer 在后续里程碑统一处理） |

## 发现

| # | 严重程度 | 描述 | 处理 |
|---|---------|------|------|
| 1 | 🟡 小问题 | 依赖计划中 `hmac` 未在 workspace 声明 | ✅ 已修正：里程碑文档标注需新增 |
| 2 | 🟡 小问题 | 依赖计划中 `chrono` 非必要，workspace 已有 `time` | ✅ 已修正：改为 `time` |

## 结论

**✅ 设计核对通过。** 里程碑文档与产品文档一致，接口设计合理，架构遵循现有模式。已修正 2 个小问题（依赖选型），无需打回。
