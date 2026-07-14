# 连接通道

## 直连资源

```text
浏览器
  ↓ WebSocket
Hub worker
  ↓ 协议 crate
目标资源
```

适用：

- 公网 SSH
- 云数据库
- 公网 S3/MinIO
- Hub 所在网络可直接访问的资源

## Agent 代理资源

```text
浏览器
  ↓ WebSocket
Hub worker
  ↓ Agent WebSocket 隧道
Agent worker
  ↓ 协议 crate
内网目标资源
```

适用：

- 家庭内网服务器
- 公司内网数据库
- 没有公网 IP 的 NAS
- 不允许开放入站端口的设备

## 通道协议

Hub 与 Agent 之间通过 WebSocket 隧道传输：

```json
{
  "type": "resource.connect",
  "payload": {
    "requestId": "req_abc",
    "resourceId": "res_ssh_1",
    "protocol": "ssh",
    "config": {}
  }
}
```

Agent 响应：

```json
{
  "type": "resource.connected",
  "payload": {
    "requestId": "req_abc",
    "channelId": "ch_123"
  }
}
```

后续数据通过 `channelId` 复用同一条 WebSocket。
