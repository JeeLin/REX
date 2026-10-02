//! Redis 控制台 API 调用封装（统一走 ApiClient，错误携带 code，自动弹 toast）

import { api } from './client'

export interface DbInfo {
  index: number
  keys: number
  expires: number
}

export interface KeyInfo {
  key: string
  type_name: string
}

export interface RedisInfo {
  redis_version: string
  os: string
  process_id: string
  connected_clients: string
  used_memory: string
  used_memory_peak: string
  total_commands_processed: string
  keyspace: { db: string; keys: number; expires: number }[]
}

export interface FormatInfo {
  detected: string
  decoded?: string
  compression?: string
}

export interface RedisStringValue {
  value: string
  format?: FormatInfo
}

export interface RedisValue {
  type: 'String' | 'List' | 'Set' | 'ZSet' | 'Hash' | 'Stream'
  value: unknown
}

export async function connect(resourceId: string): Promise<string> {
  const data = await api.post<{ session_id: string }>('/redis/connect', { resource_id: resourceId })
  return data.session_id
}

export async function disconnect(sessionId: string): Promise<void> {
  await api.post<{ ok: boolean }>('/redis/disconnect', { session_id: sessionId })
}

export async function getDatabases(sessionId: string): Promise<DbInfo[]> {
  return api.get<DbInfo[]>('/redis/databases', { session_id: sessionId })
}

export async function selectDb(sessionId: string, db: number): Promise<void> {
  await api.post<{ ok: boolean }>('/redis/select', { session_id: sessionId, db })
}

export async function scan(sessionId: string, pattern = '*', count = 100): Promise<KeyInfo[]> {
  return api.get<KeyInfo[]>('/redis/scan', {
    session_id: sessionId,
    pattern,
    count: String(count),
  })
}

export async function getValue(sessionId: string, key: string): Promise<RedisValue> {
  return api.get<RedisValue>('/redis/key', { session_id: sessionId, key })
}

export async function setValue(sessionId: string, key: string, value: string): Promise<void> {
  await api.post<{ ok: boolean }>('/redis/set', { session_id: sessionId, key, value })
}

export async function delKeys(sessionId: string, keys: string[]): Promise<number> {
  const data = await api.post<{ deleted: number }>('/redis/del', { session_id: sessionId, keys })
  return data.deleted
}

export async function getTtl(sessionId: string, key: string): Promise<number> {
  const data = await api.get<{ ttl: number }>('/redis/ttl', { session_id: sessionId, key })
  return data.ttl
}

export async function setTtl(sessionId: string, key: string, seconds: number): Promise<void> {
  await api.post<{ ok: boolean }>('/redis/set-ttl', { session_id: sessionId, key, seconds })
}

export async function getInfo(sessionId: string): Promise<RedisInfo> {
  return api.get<RedisInfo>('/redis/info', { session_id: sessionId })
}

export async function runCommand(sessionId: string, args: string[]): Promise<string> {
  const data = await api.post<{ result: string }>('/redis/command', { session_id: sessionId, args })
  return data.result
}

export interface PubSubMessage {
  channel: string
  data: string
}

export async function pubsubPoll(sessionId: string, channels: string[], timeoutMs = 5000): Promise<PubSubMessage[]> {
  const data = await api.post<{ messages: PubSubMessage[] }>('/redis/pubsub/poll', {
    session_id: sessionId,
    channels,
    timeout_ms: timeoutMs,
  })
  return data.messages
}
