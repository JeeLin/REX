// ── 类型定义 ──────────────────────────────────────────────

/** Redis 响应值类型（与后端 RedisValue 枚举一致） */
export type RedisValue =
  | { type: 'Status'; value: string }
  | { type: 'Error'; value: string }
  | { type: 'Integer'; value: number }
  | { type: 'Bulk'; value: string | null }
  | { type: 'Array'; value: RedisValue[] }
  | { type: 'Null' }

/** Redis 命令执行响应 */
export interface RedisResponse {
  value: RedisValue
  elapsed_ms: number
}

/** WebSocket 命令消息（客户端 → Hub） */
export interface RedisCommandMsg {
  type: 'command'
  id: string
  command: string
}

/** WebSocket 响应消息（Hub → 客户端） */
export interface RedisResponseMsg {
  type: 'response'
  id: string
  value: RedisValue
  elapsed_ms: number
}

export interface RedisErrorMsg {
  type: 'error'
  id: string
  message: string
}

export interface RedisConnectedMsg {
  type: 'connected'
  server: Record<string, string>
}

export interface RedisDisconnectedMsg {
  type: 'disconnected'
  reason: string
}

export type RedisServerMsg =
  | RedisResponseMsg
  | RedisErrorMsg
  | { type: 'pong' }
  | RedisConnectedMsg
  | RedisDisconnectedMsg

/** 历史记录条目 */
export interface RedisHistoryEntry {
  id: string
  command: string
  timestamp: number
}
