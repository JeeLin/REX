// ── Docker 容器信息类型 ──────────────────────────────────────

/** 容器状态枚举（与后端 ContainerState 一致） */
export type DockerContainerState = 'Running' | 'Paused' | 'Stopped' | 'Created' | 'Dead'

/** 端口映射 */
export interface DockerPortMapping {
  private: number
  public: number | null
  protocol: string
}

/** 容器信息 */
export interface DockerContainerInfo {
  id: string
  name: string
  image: string
  state: DockerContainerState
  status: string
  created: string
  ports: DockerPortMapping[]
}

// ── WebSocket 消息协议（Hub → 客户端）──────────────────────

/** 操作结果 */
export interface DockerResponseMsg {
  type: 'response'
  id: string
  data: unknown
}

/** 操作错误 */
export interface DockerErrorMsg {
  type: 'error'
  id: string
  message: string
}

/** 已连接到 Docker daemon */
export interface DockerConnectedMsg {
  type: 'connected'
  server: Record<string, string>
}

/** 连接断开 */
export interface DockerDisconnectedMsg {
  type: 'disconnected'
  reason: string
}

/** 服务端消息联合类型 */
export type DockerServerMsg =
  | DockerResponseMsg
  | DockerErrorMsg
  | { type: 'pong' }
  | DockerConnectedMsg
  | DockerDisconnectedMsg
