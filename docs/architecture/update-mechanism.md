# 更新机制

## 更新状态文件

### 路径

```text
{data-dir}/update-state.json
```

### Schema

```json
{
  "phase": "idle",
  "target_version": "",
  "old_version": "v0.1.0",
  "staged_path": "",
  "rollback_path": "",
  "attempt": 0
}
```

### 状态枚举

| 状态 | 说明 |
|------|------|
| `idle` | 无更新，正常运行 |
| `requested` | worker 已下载新版本并写入状态，请求 supervisor 重启 |
| `starting_new` | supervisor 正在替换二进制并启动新版 worker |
| `committed` | 新版本健康检查通过，更新完成，删除旧备份 |
| `rolling_back` | 新版本健康检查失败，supervisor 正在恢复旧版 |
| `rolled_back` | 回滚完成，旧版 worker 正在运行 |
| `failed` | 连续 3 次启动失败，更新终止，保留旧版 |

### 状态流转

```text
idle
  ↓ (worker 下载新版)
requested
  ↓ (supervisor 替换二进制)
starting_new
  ↓ (健康检查通过)          ↓ (健康检查失败, attempt < 3)    ↓ (attempt >= 3)
committed               rolling_back                    rolling_back
  ↓                       ↓                                ↓
idle                    重启 worker                       rolled_back
                          ↓                                  ↓
                        committed / failed                  idle
```

---

## 更新实现细节

### Hub Release 结构

GitHub Release 建议包含：

```text
rex-hub-v0.2.0-linux-amd64
rex-hub-v0.2.0-linux-arm64
rex-hub-v0.2.0-darwin-arm64
rex-hub-v0.2.0-darwin-amd64
rex-hub-v0.2.0-windows-amd64.exe
rex-hub-v0.2.0-SHA256SUMS
```

Hub worker 下载对应平台二进制和 `SHA256SUMS`，校验通过后才进入替换流程。

### Agent 下载包

Hub 部署包需要包含同版本 Agent 二进制：

```text
agent-binaries/
├── rex-agent-linux-amd64
├── rex-agent-linux-arm64
├── rex-agent-linux-armv7l
├── rex-agent-darwin-arm64
├── rex-agent-darwin-amd64
└── rex-agent-windows-amd64.exe
```

Hub 在 `/api/agent/download` 中读取对应文件，并返回 SHA256。

### 原子替换

Unix/Linux/macOS：

```text
当前二进制：/opt/rex-hub/rex-hub
staging：/opt/rex-hub/update/staging/rex-hub.v0.2.0
rollback：/opt/rex-hub/update/rollback/rex-hub.v0.1.0
```

替换流程：

```text
rename 当前二进制 → rollback
rename staging → 当前二进制
chmod +x 当前二进制
启动新 worker
```

### 更新状态写入要求

`update-state.json` 必须原子写入，避免 supervisor 读到半写入文件：

```text
写 update-state.json.tmp
fsync 临时文件
rename update-state.json.tmp → update-state.json
```

### 避免更新死循环

父进程启动新 worker 时：

```text
REX_UPDATE_PENDING=1
REX_TARGET_VERSION=v0.2.0
```

worker 检测到 `REX_UPDATE_PENDING=1` 时：

- 不检查 GitHub / Hub 更新。
- 只执行健康检查。
- 健康通过后写入 `phase=committed`。
- 健康失败则退出，让 supervisor 增加 attempt。
