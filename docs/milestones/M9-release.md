# M9: 打包与发布

## Context

M0-M8 完成了全部功能开发。M9 补充 CI/CD 缺失环节，形成可发布产物。

## 产品边界

**做什么：**
- Hub 多平台构建（linux/amd64, linux/arm64, mac/amd64, mac/arm64, windows/amd64）
- Release 包含 Hub + Agent 所有平台二进制
- SHA256SUMS 校验文件
- Docker Hub 镜像嵌入前端 + Agent 二进制
- Docker Agent 镜像（linux/amd64 + linux/arm64）
- Release 说明（changelog）
- 升级说明文档

**不做什么：**
- Windows Docker 镜像
- macOS/Windows Docker 镜像
- 自动发布到 npm/crates.io
- 代码签名

---

## 子任务清单

| # | 内容 | 前端/后端 | 状态 |
|---|------|-----------|------|
| 9.1 | Hub 多平台构建 | 后端 | ✅ |
| 9.2 | Release 包含 Hub 二进制 + SHA256SUMS | 后端 | ✅ |
| 9.3 | Release 说明模板 | 文档 | ✅ |
| 9.4 | 升级说明文档 | 文档 | ✅ |

---

## 子任务 9.1：Hub 多平台构建

### 功能目标

扩展 CI 的 `build-hub` job 为多平台矩阵，覆盖 linux/amd64、linux/arm64、mac/amd64、mac/arm64、windows/amd64。

### 修改文件

```text
.github/workflows/ci.yml    修改：build-hub 改为矩阵构建
```

### 矩阵配置

```yaml
build-hub:
  needs: [check-rust, build-frontend]
  strategy:
    matrix:
      include:
        - target: x86_64-unknown-linux-gnu
          os: ubuntu-latest
          arch: linux-amd64
        - target: aarch64-unknown-linux-gnu
          os: ubuntu-latest
          arch: linux-arm64
          cross: true
        - target: x86_64-apple-darwin
          os: macos-14
          arch: mac-amd64
        - target: aarch64-apple-darwin
          os: macos-14
          arch: mac-arm64
        - target: x86_64-pc-windows-msvc
          os: windows-latest
          arch: windows-amd64
```

### 测试标准

- `cargo fmt --check` + `cargo clippy` 通过
- 构建产物命名：`rex-hub-{arch}`

### 提交信息

```
ci: add multi-platform Hub build matrix
```

---

## 子任务 9.2：Release 包含 Hub 二进制 + SHA256SUMS

### 功能目标

Release job 下载所有 Hub 二进制，生成 SHA256SUMS，和 Agent zip 一起发布到 GitHub Release。

### 修改文件

```text
.github/workflows/ci.yml    修改：release job 增加 Hub 下载 + SHA256SUMS
```

### Release 流程

```text
1. 下载所有 agent-* artifact
2. 下载所有 hub-* artifact
3. 打包 agent 为 zip（现有逻辑）
4. 打包 hub 为 zip（新增）
5. 生成 SHA256SUMS 文件
6. 发布到 GitHub Release（hub zip + agent zip + SHA256SUMS）
```

### SHA256SUMS 格式

```text
<sha256hash>  rex-hub-linux-amd64.zip
<sha256hash>  rex-hub-linux-arm64.zip
<sha256hash>  rex-hub-mac-amd64.zip
<sha256hash>  rex-hub-mac-arm64.zip
<sha256hash>  rex-hub-windows-amd64.zip
<sha256hash>  rex-agent-linux-amd64.zip
<sha256hash>  rex-agent-linux-arm64.zip
<sha256hash>  rex-agent-mac-amd64.zip
<sha256hash>  rex-agent-mac-arm64.zip
<sha256hash>  rex-agent-windows-amd64.zip
```

### 测试标准

- Release 包含所有平台 hub 和 agent zip
- SHA256SUMS 包含所有 zip 的校验和

### 提交信息

```
ci: add hub binaries and SHA256SUMS to release
```

---

## 子任务 9.3：Release 说明模板

### 功能目标

在 CI 中或仓库中维护 Release 说明模板，每次 Release 自动生成 changelog。

### 修改文件

```text
docs/RELEASE-TEMPLATE.md    新增：Release 说明模板
```

### 模板内容

```markdown
# REX Hub v{version}

## 变更

### 新增
- ...

### 修复
- ...

### 改进
- ...

## 下载

### Hub（管理面板 + 后端）
| 平台 | 文件 |
|------|------|
| Linux amd64 | `rex-hub-linux-amd64.zip` |
| Linux arm64 | `rex-hub-linux-arm64.zip` |
| macOS Intel | `rex-hub-mac-amd64.zip` |
| macOS Apple Silicon | `rex-hub-mac-arm64.zip` |
| Windows | `rex-hub-windows-amd64.zip` |

### Agent（内网代理）
| 平台 | 文件 |
|------|------|
| Linux amd64 | `rex-agent-linux-amd64.zip` |
| Linux arm64 | `rex-agent-linux-arm64.zip` |
| macOS Intel | `rex-agent-mac-amd64.zip` |
| macOS Apple Silicon | `rex-agent-mac-arm64.zip` |
| Windows | `rex-agent-windows-amd64.zip` |

### Docker

\```bash
# Hub
docker pull ghcr.io/anthropics/rex-hub:v{version}

# Agent
docker pull ghcr.io/anthropics/rex-agent:v{version}
\```

### 校验

下载 `SHA256SUMS` 文件验证完整性：
\```bash
sha256sum -c SHA256SUMS --ignore-missing
\```

## 升级

参考 [升级说明](docs/UPGRADE.md)。
```

### 提交信息

```
docs: add release notes template
```

---

## 子任务 9.4：升级说明文档

### 功能目标

编写用户升级说明，覆盖 Docker 部署和二进制部署两种方式。

### 修改文件

```text
docs/UPGRADE.md    新增：升级说明
```

### 内容覆盖

1. Docker 部署升级流程
2. 二进制部署升级流程
3. 备份建议
4. 回滚方法

### 提交信息

```
docs: add upgrade guide
```

## 设计核对点

- [ ] CI 构建所有平台 Hub 二进制
- [ ] Release 包含 Hub + Agent 所有平台二进制
- [ ] SHA256SUMS 校验文件包含在 Release 中
- [ ] Docker 镜像可正常启动
- [ ] Release 说明包含下载指引

## Flow Status

- [x] 步骤1：编写里程碑文档
- [x] 步骤2：设计核对
- [x] 步骤3：开发
- [x] 步骤4：代码精简
- [x] 步骤5：代码审查
- [x] 步骤6：测试验证
- [x] 步骤7：设计再确认
- [x] 步骤8：提交
