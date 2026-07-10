import type { RedisValue } from '@/api/redis'

export interface KeyWithType {
  key: string
  type: string
  ttl?: number // -2=expired, -1=no expiry, >=0 seconds remaining
}

export interface OutputEntry {
  id: number
  command: string
  response?: RedisValue
  error?: string
  elapsed_ms?: number
}
