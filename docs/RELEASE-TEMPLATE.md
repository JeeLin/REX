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

```bash
# Hub
docker pull ghcr.io/anthropics/rex-hub:v{version}

# Agent
docker pull ghcr.io/anthropics/rex-agent:v{version}
```

### 校验

下载 `SHA256SUMS` 文件验证完整性：
```bash
sha256sum -c SHA256SUMS --ignore-missing
```

## 升级

参考 [升级说明](UPGRADE.md)。
